//! 贝叶斯优化求解器（V3.0 口径，替代 V2.2 说明书的遗传算法）
//!
//! 依据：http://150.158.94.206 绿电直连新能源优化配置 V3.0 实测——
//! 界面「算法参数」区仅有「总评估次数」（默认 100）与「初始随机采样点数」（默认 20）
//! 两项，V2.2 说明书中的遗传代数 / 交叉概率 / 变异概率 / 种群大小已不存在，
//! 软件标注为「贝叶斯优化，测试中……」。
//!
//! 实现（纯 Rust，不依赖 BLAS/LAPACK，问题规模仅 3 维 × 100 次评估）：
//! - 决策变量 3 维（风电 kW / 光伏 kW / 储能 kWh），归一化到 [0,1]³
//! - 初始设计：拉丁超立方采样（LHS），固定种子，可复现
//! - 代理模型：高斯过程回归，常值趋势 + Matern 5/2 核（ARD 逐维长度尺度）
//!   + 观测噪声；超参按对数边际似然在网格上择优
//! - 采集函数：gp_hedge——在 EI / LCB / PI 三者间按 Hedge 赌博机自适应选择，
//!   与 scikit-optimize 默认口径同构（V3.0 为 Python 实现，默认即 gp_hedge）。
//!   单一 EI 在目标函数较平坦时易过早收敛到局部盆地，混入 LCB 的乐观探索
//!   与 PI 的强开发可显著提升大择优范围下的稳健性。
//! - 采集函数寻优：LHS 候选集 + 围绕当前最优的局部扰动精化
//! - 与 GA 一致：固定随机种子 → 相同参数得到相同结果（可复现 / 可对拍）

use crate::compute::ga::Genes;
use crate::compute::params::BoParams;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// 固定随机种子：与 GA 同策略，保证结果可复现
const BO_SEED: u64 = 0x2026_0830_0001;
/// 采集函数候选集规模（全局 LHS 部分）
const N_CANDIDATES: usize = 3072;
/// 局部扰动精化步数
const N_REFINE: usize = 96;
/// ARD 长度尺度坐标下降轮数
const ARD_SWEEPS: usize = 2;
/// EI / PI 的探索参数 ξ
const XI: f64 = 0.01;
/// LCB 的探索系数 κ（下置信界：μ − κσ，κ 越大越偏探索）
const KAPPA: f64 = 1.96;
/// gp_hedge 的 Hedge 学习率 η（softmax 温度倒数）
const HEDGE_ETA: f64 = 1.0;
/// 预算末段转入纯开发的比例（后 25% 的评估只做开发，不再探索）
const EXPLOIT_TAIL: f64 = 0.25;


/// 超参网格：长度尺度（单位立方口径）
///
/// 上界需覆盖到 2.0：目标函数在归一化后的 [0,1]³ 上通常相当平滑，
/// 长度尺度的对数边际似然最优点常落在 1.0 以上，网格过窄会把超参压在边界上。
const LENGTH_SCALES: [f64; 9] = [0.05, 0.1, 0.2, 0.35, 0.5, 0.75, 1.0, 1.5, 2.0];
/// 超参网格：观测噪声方差
const NOISES: [f64; 2] = [1e-6, 1e-3];

// ---------------------------------------------------------------- 数值工具

/// 标准正态 PDF
fn norm_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// 互补误差函数（Numerical Recipes erfcc，|误差| < 1.2e-7）
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 2.0 / (2.0 + z);
    let ans = t
        * (-z * z - 1.265_512_23
            + t * (1.000_023_68
                + t * (0.374_091_96
                    + t * (0.096_784_18
                        + t * (-0.186_288_06
                            + t * (0.278_868_07
                                + t * (-1.135_203_98
                                    + t * (1.488_515_87
                                        + t * (-0.822_152_23 + t * 0.170_872_77)))))))))
        .exp();
    if x >= 0.0 { ans } else { 2.0 - ans }
}

/// 标准正态 CDF
fn norm_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

/// Cholesky 分解：返回下三角 L 使 L·Lᵀ = A（A 对称正定）
fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = a[i][j];
            for k in 0..j {
                s -= l[i][k] * l[j][k];
            }
            if i == j {
                if s <= 0.0 {
                    return None;
                }
                l[i][j] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    Some(l)
}

/// 解下三角方程组 L·x = b
fn solve_lower(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.0f64; n];
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i][k] * x[k];
        }
        x[i] = s / l[i][i];
    }
    x
}

/// 解上三角方程组 Lᵀ·x = b（L 为下三角）
fn solve_lower_transpose(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.0f64; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in (i + 1)..n {
            s -= l[k][i] * x[k];
        }
        x[i] = s / l[i][i];
    }
    x
}

// ---------------------------------------------------------------- 高斯过程

/// 固定超参下的拟合结果
struct GpFit {
    /// 对数边际似然
    lml: f64,
    l: Vec<Vec<f64>>,
    alpha: Vec<f64>,
    /// 常值趋势项（标准化量纲），由广义最小二乘估计
    mean: f64,
}

/// 高斯过程回归（常值趋势 + Matern 5/2 核，ARD 逐维长度尺度）
///
/// - **常值趋势（普通克里金）**：目标函数（综合电价 0.2~0.6 元/kWh、初投资
///   数万元）远离 0，纯零均值先验会系统性失配；趋势项由 GLS 闭式估计：
///   μ̂ = (1ᵀK⁻¹y) / (1ᵀK⁻¹1)。
/// - **ARD 逐维长度尺度**：三个决策变量对目标的影响尺度差异极大——
///   风电容量高度敏感，储能容量在部分电价结构下几乎不影响目标函数。
///   共享长度尺度无法刻画这种各向异性，会显著削弱代理模型质量。
struct Gp {
    xs: Vec<[f64; 3]>,
    y_mean: f64,
    y_std: f64,
    /// 常值趋势项（标准化量纲）
    mean: f64,
    length_scales: [f64; 3],
    /// L：K + σ²I 的 Cholesky 下三角
    l: Vec<Vec<f64>>,
    /// α = (K + σ²I)⁻¹ (y − μ̂)
    alpha: Vec<f64>,
}

impl Gp {
    /// Matern 5/2 核（ARD）
    fn kernel_ls(a: &[f64; 3], b: &[f64; 3], ls: &[f64; 3]) -> f64 {
        let mut d2 = 0.0;
        for j in 0..3 {
            let diff = (a[j] - b[j]) / ls[j];
            d2 += diff * diff;
        }
        let d = d2.sqrt();
        let s5 = 5.0f64.sqrt();
        (1.0 + s5 * d + (5.0 / 3.0) * d2) * (-s5 * d).exp()
    }

    fn kernel(&self, a: &[f64; 3], b: &[f64; 3]) -> f64 {
        Gp::kernel_ls(a, b, &self.length_scales)
    }

    /// 固定超参拟合（普通克里金）：返回（对数边际似然, L, α, 常值趋势）
    fn fit_fixed(xs: &[[f64; 3]], ys: &[f64], ls: &[f64; 3], noise: f64) -> Option<GpFit> {
        let n = xs.len();
        let mut k = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            for j in 0..=i {
                let v = Gp::kernel_ls(&xs[i], &xs[j], ls);
                k[i][j] = v;
                k[j][i] = v;
            }
            k[i][i] += noise + 1e-8;
        }
        let l = cholesky(&k)?;

        // 常值趋势项的 GLS 闭式解：μ̂ = (1ᵀK⁻¹y) / (1ᵀK⁻¹1)
        let ones = vec![1.0f64; n];
        let kinv_ones = solve_lower_transpose(&l, &solve_lower(&l, &ones));
        let denom: f64 = kinv_ones.iter().sum();
        let mean = if denom.abs() > 1e-12 {
            kinv_ones.iter().zip(ys.iter()).map(|(a, y)| a * y).sum::<f64>() / denom
        } else {
            0.0
        };

        // 去趋势后求解 α
        let yc: Vec<f64> = ys.iter().map(|y| y - mean).collect();
        let alpha = solve_lower_transpose(&l, &solve_lower(&l, &yc));

        // 对数边际似然：-½ (y−μ̂)ᵀK⁻¹(y−μ̂) − ½ log|K| − n/2 log 2π
        let data_fit: f64 = yc.iter().zip(alpha.iter()).map(|(y, a)| y * a).sum();
        let log_det: f64 = (0..n).map(|i| l[i][i].ln()).sum::<f64>() * 2.0;
        let lml = -0.5 * data_fit - 0.5 * log_det - 0.5 * n as f64 * (2.0 * std::f64::consts::PI).ln();
        if !lml.is_finite() {
            return None;
        }
        Some(GpFit { lml, l, alpha, mean })
    }

    /// 拟合：先网格搜共享长度尺度，再按坐标下降逐维细化（ARD）
    fn fit(xs: Vec<[f64; 3]>, ys_raw: &[f64]) -> Option<Self> {
        let n = xs.len();
        if n == 0 || n != ys_raw.len() {
            return None;
        }

        // 观测值标准化（GP 零均值先验的前提）
        let y_mean = ys_raw.iter().sum::<f64>() / n as f64;
        let var = ys_raw.iter().map(|y| (y - y_mean) * (y - y_mean)).sum::<f64>() / n as f64;
        let y_std = var.sqrt().max(1e-12);
        let ys: Vec<f64> = ys_raw.iter().map(|y| (y - y_mean) / y_std).collect();

        // ---- 阶段一：共享长度尺度粗搜 ----
        let mut best_ls = [LENGTH_SCALES[2]; 3];
        let mut best_noise = NOISES[0];
        let mut best: Option<GpFit> = None;
        let mut best_lml = f64::NEG_INFINITY;
        for &ls in &LENGTH_SCALES {
            for &noise in &NOISES {
                if let Some(fit) = Gp::fit_fixed(&xs, &ys, &[ls; 3], noise) {
                    if fit.lml > best_lml {
                        best_lml = fit.lml;
                        best_ls = [ls; 3];
                        best_noise = noise;
                        best = Some(fit);
                    }
                }
            }
        }

        // ---- 阶段二：逐维坐标下降（ARD）----
        for _ in 0..ARD_SWEEPS {
            let mut improved = false;
            for j in 0..3 {
                for &cand in &LENGTH_SCALES {
                    if (cand - best_ls[j]).abs() < 1e-12 {
                        continue;
                    }
                    let mut trial = best_ls;
                    trial[j] = cand;
                    if let Some(fit) = Gp::fit_fixed(&xs, &ys, &trial, best_noise) {
                        if fit.lml > best_lml + 1e-9 {
                            best_lml = fit.lml;
                            best_ls = trial;
                            best = Some(fit);
                            improved = true;
                        }
                    }
                }
            }
            if !improved {
                break;
            }
        }

        let fit = best?;
        Some(Gp {
            xs,
            y_mean,
            y_std,
            mean: fit.mean,
            length_scales: best_ls,
            l: fit.l,
            alpha: fit.alpha,
        })
    }

    /// 后验均值与标准差（原始量纲）
    fn predict(&self, x: &[f64; 3]) -> (f64, f64) {
        let n = self.xs.len();
        let mut ks = vec![0.0f64; n];
        for i in 0..n {
            ks[i] = self.kernel(&self.xs[i], x);
        }
        // 普通克里金：μ(x) = μ̂ + k*ᵀK⁻¹(y − μ̂1)
        let mu_std: f64 =
            self.mean + ks.iter().zip(self.alpha.iter()).map(|(k, a)| k * a).sum::<f64>();
        let v = solve_lower(&self.l, &ks);
        let var = (self.kernel(x, x) - v.iter().map(|x| x * x).sum::<f64>()).max(0.0);
        (mu_std * self.y_std + self.y_mean, var.sqrt() * self.y_std)
    }

    /// 期望改善 EI（原始量纲，越大越优）
    fn expected_improvement(&self, x: &[f64; 3], y_best: f64) -> f64 {
        let (mu, sigma) = self.predict(x);
        if sigma <= 1e-12 {
            return 0.0;
        }
        let improvement = y_best - mu - XI;
        let z = improvement / sigma;
        improvement * norm_cdf(z) + sigma * norm_pdf(z)
    }

    /// 改善概率 PI（原始量纲，越大越优）
    fn probability_of_improvement(&self, x: &[f64; 3], y_best: f64) -> f64 {
        let (mu, sigma) = self.predict(x);
        if sigma <= 1e-12 {
            return 0.0;
        }
        let z = (y_best - mu - XI) / sigma;
        norm_cdf(z)
    }

    /// 下置信界 LCB 的负值（原始量纲，越大越优；对应最小化 μ − κσ）
    fn lower_confidence_bound(&self, x: &[f64; 3]) -> f64 {
        let (mu, sigma) = self.predict(x);
        KAPPA * sigma - mu
    }
}

/// 采集函数种类（gp_hedge 的"臂"）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AcqKind {
    Ei,
    Lcb,
    Pi,
    /// 纯开发：直接取后验均值最小处。用于预算末段收敛，不再探索。
    Exploit,
}

const ACQ_KINDS: [AcqKind; 3] = [AcqKind::Ei, AcqKind::Lcb, AcqKind::Pi];

impl AcqKind {
    /// 采集函数值（越大越优）
    fn value(self, gp: &Gp, x: &[f64; 3], y_best: f64) -> f64 {
        match self {
            AcqKind::Ei => gp.expected_improvement(x, y_best),
            AcqKind::Lcb => gp.lower_confidence_bound(x),
            AcqKind::Pi => gp.probability_of_improvement(x, y_best),
            AcqKind::Exploit => -gp.predict(x).0,
        }
    }
}

/// gp_hedge 选择器：把采集函数的选择建模为 3 臂赌博机
///
/// 每次迭代按 softmax(η · 累计增益) 采样一个臂；评估后以
/// `gain = max(0, 改进前最优值 − 本次观测值)` 更新该臂的累计增益。
/// 长期看会自适应地偏好带来实际改善的采集函数。
struct GpHedge {
    gains: [f64; 3],
    used: [u32; 3],
}

impl GpHedge {
    fn new() -> Self {
        Self { gains: [0.0; 3], used: [0; 3] }
    }

    /// 按 softmax 采样一个臂
    fn sample(&self, rng: &mut StdRng) -> usize {
        let max_g = self.gains.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let weights: Vec<f64> = self
            .gains
            .iter()
            .map(|g| (HEDGE_ETA * (g - max_g)).exp())
            .collect();
        let total: f64 = weights.iter().sum();
        if !total.is_finite() || total <= 0.0 {
            return rng.gen_range(0..3);
        }
        let mut t = rng.gen::<f64>() * total;
        for (i, w) in weights.iter().enumerate() {
            t -= w;
            if t <= 0.0 {
                return i;
            }
        }
        2
    }

    fn update(&mut self, arm: usize, gain: f64) {
        self.gains[arm] += gain.max(0.0);
        self.used[arm] += 1;
    }
}

// ---------------------------------------------------------------- 采样与设计

/// 拉丁超立方采样：n 个点 × 3 维，返回 [0,1]³
fn lhs(rng: &mut StdRng, n: usize) -> Vec<[f64; 3]> {
    let mut out = vec![[0.0f64; 3]; n];
    for j in 0..3 {
        // 每维切成 n 个等宽区间，各区间内取一个随机点后打乱
        let mut perm: Vec<f64> = (0..n)
            .map(|i| (i as f64 + rng.gen::<f64>()) / n as f64)
            .collect();
        for i in (1..n).rev() {
            let k = rng.gen_range(0..=i);
            perm.swap(i, k);
        }
        for i in 0..n {
            out[i][j] = perm[i];
        }
    }
    out
}

/// 单个均匀随机点
fn random_unit(rng: &mut StdRng) -> [f64; 3] {
    [rng.gen(), rng.gen(), rng.gen()]
}

/// 反归一化：[0,1]³ → 真实量纲
fn from_unit(u: [f64; 3], bounds: &[(f64, f64); 3]) -> Genes {
    let mut g = [0.0f64; 3];
    for i in 0..3 {
        g[i] = bounds[i].0 + u[i].clamp(0.0, 1.0) * (bounds[i].1 - bounds[i].0);
    }
    g
}

/// 在单位立方内生成候选点：全局 LHS + 围绕最优点的局部扰动
fn candidates(rng: &mut StdRng, best_u: [f64; 3], n_global: usize) -> Vec<[f64; 3]> {
    let mut out: Vec<[f64; 3]> = lhs(rng, n_global);
    // 局部扰动：以当前最优为中心，多尺度混合（粗探索 + 细开发）
    let scales = [0.4f64, 0.2, 0.1, 0.05, 0.02];
    let per_scale = N_CANDIDATES / (scales.len() * 2);
    for s in scales {
        for _ in 0..per_scale {
            let mut u = best_u;
            for j in 0..3 {
                let delta = (rng.gen::<f64>() - 0.5) * 2.0 * s;
                u[j] = (u[j] + delta).clamp(0.0, 1.0);
            }
            out.push(u);
        }
    }
    out
}

// ---------------------------------------------------------------- 主流程

/// 单方案评估结果：`(目标函数值, 约束违反量)`
///
/// 违反量为 0 表示满足全部约束；越大表示违反越严重（比例单位）。
pub type EvalOut = (f64, f64);

/// 运行贝叶斯优化，返回（最优基因，最优适应度）
///
/// **约束贝叶斯优化**：目标函数与约束违反量分别建模——
/// - 目标 GP：拟合全部观测的目标值
/// - 违反量 GP：拟合违反量，给出 P(可行) = Φ(−μ_v/σ_v)
/// - 采集函数：EI × P(可行)，即"期望改善"按可行性概率加权
///
/// 之所以不用「目标 + 罚项」的单一代理模型：实测择优范围取 1~500 时
/// 可行域仅约 19%，20 个初始采样点里只有约 4 个可行，单一 GP 拟合到的
/// 主要是罚值悬崖，采集函数会被引向不可行区，最终收敛到局部解。
/// 分离建模后，优化器能同时感知"哪儿目标低"与"哪儿可行"。
///
/// - `bo`：总评估次数 n_iter 与初始随机采样点数 n_init
/// - `evaluate`：返回 (目标值, 违反量)，串行调用（每步依赖上一步的代理模型）
/// - `on_iteration`：每次评估后回调（已评估次数, 当前最优可行目标值）
/// - `is_cancelled`：每步检查，返回 true 时终止并返回 Err("计算已取消")
pub fn optimize(
    bounds: [(f64, f64); 3],
    bo: &BoParams,
    evaluate: &(dyn Fn(Genes) -> EvalOut + Sync),
    on_iteration: &dyn Fn(u32, f64),
    is_cancelled: &dyn Fn() -> bool,
) -> Result<(Genes, f64), String> {
    let mut rng = StdRng::seed_from_u64(BO_SEED);
    let n_init = (bo.n_init as usize).max(2);
    let n_iter = (bo.n_iter as usize).max(n_init + 1);

    let mut xs: Vec<[f64; 3]> = Vec::with_capacity(n_iter);
    let mut ys: Vec<f64> = Vec::with_capacity(n_iter);
    let mut vs: Vec<f64> = Vec::with_capacity(n_iter);

    // 记录可行解中的最优者（下标, 目标值）
    let mut best_feas: Option<(usize, f64)> = None;
    let push = |xs: &mut Vec<[f64; 3]>,
                   ys: &mut Vec<f64>,
                   vs: &mut Vec<f64>,
                   best_feas: &mut Option<(usize, f64)>,
                   u: [f64; 3],
                   y: f64,
                   v: f64| {
        let i = xs.len();
        xs.push(u);
        ys.push(y);
        vs.push(v);
        if v <= 0.0 && y.is_finite() {
            if best_feas.map_or(true, |(_, b)| y < b) {
                *best_feas = Some((i, y));
            }
        }
    };

    // ---- 阶段一：初始随机采样（拉丁超立方）----
    let init_design = lhs(&mut rng, n_init);
    for u in init_design {
        if is_cancelled() {
            return Err("计算已取消".to_string());
        }
        let (y, v) = evaluate(from_unit(u, &bounds));
        if y.is_finite() {
            push(&mut xs, &mut ys, &mut vs, &mut best_feas, u, y, v);
        }
        on_iteration(xs.len() as u32, best_feas.map_or(f64::INFINITY, |(_, b)| b));
    }

    if xs.is_empty() {
        return Err("初始采样点全部无效，请检查择优范围与参数".to_string());
    }

    // ---- 阶段二：代理模型 + gp_hedge 序贯选点 ----
    let mut hedge = GpHedge::new();
    while xs.len() < n_iter {
        if is_cancelled() {
            return Err("计算已取消".to_string());
        }

        // 当前最优可行解（作为 EI 的基准）；尚无可行解时进入"可行性搜索"模式
        let y_best = best_feas.map(|(_, b)| b);
        let best_u = best_feas.map_or(xs[argmin(&vs)], |(i, _)| xs[i]);

        let gp_obj = Gp::fit(xs.clone(), &ys);
        let gp_vio = Gp::fit(xs.clone(), &vs);

        // 本轮实际使用的 gp_hedge 臂（纯开发阶段为 None），评估后据此更新增益
        let mut chosen_arm: Option<usize> = None;
        let next_u = match (&gp_obj, &gp_vio, y_best) {
            (Some(g), Some(gv), Some(yb)) => {
                // ---- 约束采集：gp_hedge 选臂，再乘上可行性概率 ----
                // 预算末段（后 EXPLOIT_TAIL 比例）固定切到纯开发，
                // 把剩余评估集中在当前最优盆地内精修，避免尾声仍在无谓探索。
                let in_tail =
                    xs.len() as f64 >= n_iter as f64 * (1.0 - EXPLOIT_TAIL);
                let arm = hedge.sample(&mut rng);
                let kind = if in_tail {
                    AcqKind::Exploit
                } else {
                    ACQ_KINDS[arm]
                };
                let scored = |u: &[f64; 3]| -> f64 {
                    let (mu_v, sd_v) = gv.predict(u);
                    // P(违反量 ≤ 0)：违反量 GP 的正态尾概率
                    let p_feas = if sd_v > 1e-12 {
                        norm_cdf(-mu_v / sd_v)
                    } else if mu_v <= 0.0 {
                        1.0
                    } else {
                        0.0
                    };
                    kind.value(g, u, yb) * p_feas.max(1e-6)
                };
                let (u, _) = maximize_acquisition(&mut rng, best_u, &scored);
                // Hedge 增益在评估后更新（纯开发阶段不计入臂的增益统计）
                if !in_tail {
                    chosen_arm = Some(arm);
                }
                u
            }
            (_, Some(gv), None) => {
                // 尚无可行解：先做可行性搜索 → 最小化违反量（最大化 −LCB(违反量)）
                let scored = |u: &[f64; 3]| -> f64 { -gv.lower_confidence_bound(u) };
                let (u, _) = maximize_acquisition(&mut rng, best_u, &scored);
                u
            }
            (Some(g), None, Some(yb)) => {
                // 违反量 GP 奇异：退化为不含可行性权重的 gp_hedge
                let arm = hedge.sample(&mut rng);
                let kind = ACQ_KINDS[arm];
                let scored = |u: &[f64; 3]| -> f64 { kind.value(g, u, yb) };
                let (u, _) = maximize_acquisition(&mut rng, best_u, &scored);
                chosen_arm = Some(arm);
                u
            }
            (None, Some(gv), Some(_)) => {
                // 目标 GP 奇异：只能做可行性搜索
                let scored = |u: &[f64; 3]| -> f64 { -gv.lower_confidence_bound(u) };
                let (u, _) = maximize_acquisition(&mut rng, best_u, &scored);
                u
            }
            _ => {
                // 其余组合（含两个代理模型都奇异）：退化为随机采样继续探索
                random_unit(&mut rng)
            }
        };

        let (y, v) = evaluate(from_unit(next_u, &bounds));
        if y.is_finite() {
            if let (Some(arm), Some(yb)) = (chosen_arm, y_best) {
                hedge.update(arm, yb - y);
            }
            push(&mut xs, &mut ys, &mut vs, &mut best_feas, next_u, y, v);
        }
        on_iteration(xs.len() as u32, best_feas.map_or(f64::INFINITY, |(_, b)| b));
    }

    log::debug!(
        "gp_hedge 臂使用次数：EI={} LCB={} PI={}（累计增益 {:?}）",
        hedge.used[0],
        hedge.used[1],
        hedge.used[2],
        hedge.gains
    );

    // 优先返回最优可行解；若全程未找到可行解，返回违反量最小者
    match best_feas {
        Some((i, y)) => Ok((from_unit(xs[i], &bounds), y)),
        None => {
            let i = argmin(&vs);
            Ok((from_unit(xs[i], &bounds), ys[i]))
        }
    }
}

/// 在候选集上最大化采集函数：全局候选 + 收缩随机搜索精化
fn maximize_acquisition(
    rng: &mut StdRng,
    best_u: [f64; 3],
    score: &dyn Fn(&[f64; 3]) -> f64,
) -> ([f64; 3], f64) {
    let cand = candidates(rng, best_u, N_CANDIDATES);
    let mut best_x = cand[0];
    let mut best_v = f64::NEG_INFINITY;
    for u in &cand {
        let v = score(u);
        if v > best_v {
            best_v = v;
            best_x = *u;
        }
    }
    let mut step = 0.1f64;
    for _ in 0..N_REFINE {
        let mut trial = best_x;
        for j in 0..3 {
            trial[j] = (trial[j] + (rng.gen::<f64>() - 0.5) * 2.0 * step).clamp(0.0, 1.0);
        }
        let v = score(&trial);
        if v > best_v {
            best_v = v;
            best_x = trial;
        }
        step *= 0.85;
    }
    (best_x, best_v)
}

fn argmin(v: &[f64]) -> usize {
    let mut i = 0;
    for k in 1..v.len() {
        if v[k] < v[i] {
            i = k;
        }
    }
    i
}

#[cfg(test)]
mod bo_internal_tests {
    use super::*;

    /// 已知解析函数：f(u) = (u0-0.7)² + (u1-0.3)² + 0.02*u2，最小点 (0.7, 0.3, 0)
    fn f(u: [f64; 3]) -> f64 {
        (u[0] - 0.7).powi(2) + (u[1] - 0.3).powi(2) + 0.02 * u[2]
    }

    /// GP 应能拟合已知曲面：在未采样点上的预测接近真值
    #[test]
    fn gp_interpolates_smooth_surface() {
        let mut rng = StdRng::seed_from_u64(7);
        let xs: Vec<[f64; 3]> = lhs(&mut rng, 60);
        let ys: Vec<f64> = xs.iter().map(|u| f(*u)).collect();
        let gp = Gp::fit(xs.clone(), &ys).expect("gp fit");

        let mut max_err = 0.0f64;
        let mut rng2 = StdRng::seed_from_u64(99);
        for u in lhs(&mut rng2, 40) {
            let (mu, _sigma) = gp.predict(&u);
            max_err = max_err.max((mu - f(u)).abs());
        }
        println!(
            "[GP] ls={:?} 最大预测误差={:.5}",
            gp.length_scales, max_err
        );
        assert!(max_err < 0.05, "GP 插值误差过大: {max_err}");
    }

    /// 采样点处 GP 应几乎无 uncertainty，且预测值≈观测值
    #[test]
    fn gp_reproduces_observations() {
        let mut rng = StdRng::seed_from_u64(11);
        let xs: Vec<[f64; 3]> = lhs(&mut rng, 40);
        let ys: Vec<f64> = xs.iter().map(|u| f(*u)).collect();
        let gp = Gp::fit(xs.clone(), &ys).expect("gp fit");
        for (x, y) in xs.iter().zip(ys.iter()) {
            let (mu, sigma) = gp.predict(x);
            assert!((mu - y).abs() < 0.05, "采样点预测 {mu} 偏离观测 {y}");
            assert!(sigma < 0.05, "采样点不确定性应接近 0，实际 {sigma}");
        }
    }

    /// 在解析函数上跑完整 BO：应收敛到真实最小点附近
    #[test]
    fn bo_finds_known_minimum() {
        let bounds = [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)];
        let evaluate = |g: [f64; 3]| -> EvalOut {
            let u = [g[0], g[1], g[2]];
            (f(u), 0.0) // 解析函数无约束，恒可行
        };
        let bo = BoParams { n_iter: 40, n_init: 8 };
        let (best, y) = optimize(bounds, &bo, &evaluate, &|_u, _f| {}, &|| false).unwrap();
        println!("[BO] 解析函数结果：{:.6} @ {:?}", y, best);
        assert!(
            y < 0.01,
            "BO 未收敛到解析最小点：y={y} @ wind={} pv={} ess={}",
            best[0], best[1], best[2]
        );
        assert!((best[0] - 0.7).abs() < 0.15, "u0 偏离: {}", best[0]);
        assert!((best[1] - 0.3).abs() < 0.15, "u1 偏离: {}", best[1]);
    }
}
