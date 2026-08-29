//! 遗传算法求解器（AR-3）
//!
//! - 实数编码（风电装机 kW / 光伏装机 kW / 储能容量 kWh）
//! - 选择：锦标赛（k=3）；交叉：模拟二进制交叉 SBX（η=15，AR-3.2）；变异：高斯扰动
//! - 精英保留（2 个），固定随机种子保证结果可复现
//! - 适应度评估使用 rayon 并行（种群 × 8760h 仿真）

use crate::compute::params::GaParams;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

/// 固定随机种子：保证相同参数得到相同结果（可复现 / 可对拍）
const GA_SEED: u64 = 0x2026_0829_0001;
const SBX_ETA: f64 = 15.0;
const TOURNAMENT_K: usize = 3;
const ELITE_COUNT: usize = 2;

pub type Genes = [f64; 3];

fn gauss(rng: &mut StdRng) -> f64 {
    let u1: f64 = rng.gen::<f64>().max(1e-12);
    let u2: f64 = rng.gen::<f64>();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

fn clamp_genes(mut g: Genes, bounds: &[(f64, f64); 3]) -> Genes {
    for i in 0..3 {
        g[i] = g[i].clamp(bounds[i].0, bounds[i].1);
    }
    g
}

fn random_individual(rng: &mut StdRng, bounds: &[(f64, f64); 3]) -> Genes {
    let mut g = [0.0f64; 3];
    for i in 0..3 {
        g[i] = rng.gen_range(bounds[i].0..=bounds[i].1);
    }
    g
}

/// 锦标赛选择：随机取 k 个个体，返回适应度最优（最小）者
fn tournament(pop: &[Genes], fits: &[f64], rng: &mut StdRng) -> Genes {
    let mut best = pop[rng.gen_range(0..pop.len())];
    let mut best_fit = f64::INFINITY;
    for _ in 0..TOURNAMENT_K {
        let i = rng.gen_range(0..pop.len());
        if fits[i] < best_fit {
            best_fit = fits[i];
            best = pop[i];
        }
    }
    best
}

/// 模拟二进制交叉（SBX）
fn sbx(p1: Genes, p2: Genes, rng: &mut StdRng) -> (Genes, Genes) {
    let mut c1 = p1;
    let mut c2 = p2;
    for i in 0..3 {
        let d = (p2[i] - p1[i]).abs();
        if d > 1e-12 && rng.gen::<f64>() < 0.5 {
            let u: f64 = rng.gen::<f64>();
            let beta = if u <= 0.5 {
                (2.0 * u).powf(1.0 / (SBX_ETA + 1.0))
            } else {
                (1.0 / (2.0 * (1.0 - u))).powf(1.0 / (SBX_ETA + 1.0))
            };
            c1[i] = 0.5 * ((1.0 + beta) * p1[i] + (1.0 - beta) * p2[i]);
            c2[i] = 0.5 * ((1.0 - beta) * p1[i] + (1.0 + beta) * p2[i]);
        }
    }
    (c1, c2)
}

/// 高斯变异：以变异概率逐基因扰动
fn mutate(g: Genes, pm: f64, rng: &mut StdRng, bounds: &[(f64, f64); 3]) -> Genes {
    let mut out = g;
    for i in 0..3 {
        if rng.gen::<f64>() < pm {
            let sigma = 0.1 * (bounds[i].1 - bounds[i].0);
            out[i] += gauss(rng) * sigma;
        }
    }
    clamp_genes(out, bounds)
}

/// 运行遗传算法，返回（最优基因，最优适应度）
///
/// - `evaluate`：适应度函数（越小越优），会被并行调用，须实现 Sync
/// - `on_generation`：每代回调（当前代数, 当代最优适应度）
/// - `is_cancelled`：每代检查，返回 true 时终止并返回 Err("计算已取消")
pub fn optimize(
    bounds: [(f64, f64); 3],
    ga: &GaParams,
    evaluate: &(dyn Fn(Genes) -> f64 + Sync),
    on_generation: &dyn Fn(u32, f64),
    is_cancelled: &dyn Fn() -> bool,
) -> Result<(Genes, f64), String> {
    let mut rng = StdRng::seed_from_u64(GA_SEED);
    let pop_size = ga.population_size as usize;

    // 初始化种群（AR-3.2：随机生成 N 个个体）
    let mut pop: Vec<Genes> = (0..pop_size)
        .map(|_| random_individual(&mut rng, &bounds))
        .collect();
    let mut fits: Vec<f64> = pop.par_iter().map(|&g| evaluate(g)).collect();

    for gen in 1..=ga.generations {
        if is_cancelled() {
            return Err("计算已取消".to_string());
        }

        // 精英保留
        let mut order: Vec<usize> = (0..pop_size).collect();
        order.sort_by(|&a, &b| fits[a].partial_cmp(&fits[b]).unwrap_or(std::cmp::Ordering::Equal));
        let mut new_pop: Vec<Genes> = order[..ELITE_COUNT.min(pop_size)]
            .iter()
            .map(|&i| pop[i])
            .collect();

        // 选择 → 交叉 → 变异
        while new_pop.len() < pop_size {
            let p1 = tournament(&pop, &fits, &mut rng);
            let p2 = tournament(&pop, &fits, &mut rng);
            let (c1, c2) = if rng.gen::<f64>() < ga.crossover_rate {
                sbx(p1, p2, &mut rng)
            } else {
                (p1, p2)
            };
            new_pop.push(mutate(c1, ga.mutation_rate, &mut rng, &bounds));
            if new_pop.len() < pop_size {
                new_pop.push(mutate(c2, ga.mutation_rate, &mut rng, &bounds));
            }
        }

        pop = new_pop;
        fits = pop.par_iter().map(|&g| evaluate(g)).collect();

        let best_fit = fits.iter().copied().fold(f64::INFINITY, f64::min);
        on_generation(gen, best_fit);
    }

    let mut best_i = 0;
    for i in 1..pop_size {
        if fits[i] < fits[best_i] {
            best_i = i;
        }
    }
    Ok((pop[best_i], fits[best_i]))
}
