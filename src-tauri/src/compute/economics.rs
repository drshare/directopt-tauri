//! 经济性计算与目标函数（AR-1）
//!
//! - 初投资 IC = C_wind×P_wind + C_pv×P_pv + C_ess×E_ess + FC
//! - 全生命周期总成本（式 1）：TC = IC + Σ OC_t/(1+r)^t + OC_x/(1+r)^x
//! - 综合电价（式 3）：p_z = TC / Σ Q_t/(1+r)^t
//! - 绿电电价（式 4）：p_g = (IC + Σ OC_g,t/(1+r)^t + OC_x/(1+r)^x) / Σ Q_g,t/(1+r)^t
//! - 年运行成本口径（AR-1.4）：方案一/方案二输配电费
//! - 逐年负荷与运行成本按单年 8760h 仿真结果外推（曲线为典型年）

use crate::compute::params::EconParams;
use crate::compute::simulate::Totals;

/// 经济性指标（单位：元，展示层负责换算万元）
#[derive(Debug, Clone)]
pub struct EconResult {
    /// 初投资 IC 元
    pub ic: f64,
    /// 年运维成本 元
    pub maint: f64,
    /// 年人员工资 元
    pub salary_cost: f64,
    /// 年电网购电成本 元（含输配电费口径）
    pub grid_buy_cost: f64,
    /// 年自发自用输配成本 元
    pub self_use_cost: f64,
    /// 年余电上网收益 元
    pub feed_revenue: f64,
    /// 展示口径年运行成本 = 购电 + 自发自用输配 + 运维 + 工资 元
    pub annual_cost_display: f64,
    /// 电池更换成本现值 OC_x/(1+r)^x 元
    pub replace_cost_pv: f64,
    /// 全生命周期总成本现值 TC 元
    pub tc: f64,
    /// 综合电价 元/kWh
    pub p_composite: f64,
    /// 绿电电价 元/kWh
    pub p_green: f64,
    /// 网电电价 元/kWh（年购电成本 / 年下网电量）
    pub p_grid: f64,
}

/// 计算经济性指标
///
/// - `wind_kw`/`pv_kw`/`ess_kwh`：装机配置；`scheme`："scheme1" | "scheme2"
pub fn compute_economics(
    t: &Totals,
    econ: &EconParams,
    scheme: &str,
    wind_kw: f64,
    pv_kw: f64,
    ess_kwh: f64,
    grid_capacity: f64,
    avg_load_rate: f64,
) -> EconResult {
    // ---- 初投资 IC（元）----
    let ic = econ.wind_invest * wind_kw + econ.pv_invest * pv_kw + econ.ess_invest * ess_kwh
        + econ.other_invest * 10_000.0;

    // ---- 年运行成本各项（元/年）----
    let maint = ic * econ.opex_ratio / 100.0;
    let salary_cost = econ.salary * 10_000.0 * econ.staff_count;

    // 年电网购电成本（AR-1.4）
    let grid_buy_cost = match scheme {
        "scheme2" => {
            // 方案二：电量电价+线损+系统运行费+基金附加 + 实际购网电量×电度输配电价
            t.grid_energy_cost + t.grid_tdu_cost
        }
        _ => {
            // 方案一：… + 接入公共电网容量×平均负荷率×8760×电度输配电价（容量法）
            let capacity_tdu = grid_capacity * (avg_load_rate / 100.0) * 8760.0 * t.avg_tdu;
            t.grid_energy_cost + capacity_tdu
        }
    };

    // 年自发自用输配成本（AR-1.4）
    let self_use_cost = match scheme {
        "scheme2" => t.self_tdu_fund_cost, // 方案二：输配电费 + 政府性基金及附加
        _ => t.self_fund_cost,             // 方案一：政府性基金及附加
    };

    let feed_revenue = t.feed_revenue;

    let annual_cost_display = grid_buy_cost + self_use_cost + maint + salary_cost;
    // 绿电口径年运行成本（不含网电购电成本）
    let oc_green = self_use_cost + maint + salary_cost - feed_revenue;
    // 计入式(3) 的年运行成本（AR-1.1：扣除余电上网收益）
    let oc = annual_cost_display - feed_revenue;

    // ---- 折现（式 1/3/4）----
    let r = (econ.discount_rate / 100.0).max(1e-9);
    let n = econ.eval_period.max(1.0);
    let mut annuity = 0.0;
    for year in 1..=(n as i64) {
        annuity += 1.0 / (1.0 + r).powi(year as i32);
    }
    let x = econ.battery_replace_year.max(1.0);
    let replace_cost =
        ess_kwh * econ.battery_replace_unit * econ.battery_replace_ratio / 100.0;
    let replace_cost_pv = replace_cost / (1.0 + r).powi(x as i32);

    // ---- 全生命周期总成本 TC（式 1）----
    let tc = ic + oc * annuity + replace_cost_pv;

    // ---- 电价（式 3/4）----
    let denom_q = t.load * annuity; // Σ Q_t/(1+r)^t
    let p_composite = if denom_q > 0.0 { tc / denom_q } else { f64::INFINITY };

    // 绿电口径供电量：负荷中由绿电覆盖部分（负荷 − 下网电量），不计入网电电量
    let load_green = (t.load - t.grid_import).max(0.0);
    let denom_qg = load_green * annuity;
    let tc_green = ic + oc_green * annuity + replace_cost_pv;
    let p_green = if denom_qg > 0.0 { tc_green / denom_qg } else { f64::INFINITY };

    let p_grid = if t.grid_import > 0.0 { grid_buy_cost / t.grid_import } else { 0.0 };

    EconResult {
        ic,
        maint,
        salary_cost,
        grid_buy_cost,
        self_use_cost,
        feed_revenue,
        annual_cost_display,
        replace_cost_pv,
        tc,
        p_composite,
        p_green,
        p_grid,
    }
}

/// 约束检查结果（AR-2.7 / 2.8 / 2.9）
#[derive(Debug, Clone)]
pub struct ConstraintStatus {
    /// 自发自用占总可用发电量比例 %
    pub self_use_gen_ratio: f64,
    /// 自发自用占总用电量比例 %
    pub self_use_load_ratio: f64,
    /// 余电上网比例 %
    pub feed_ratio: f64,
    /// 弃电率 %
    pub curtail_ratio: f64,
    /// 违反的约束说明（空 = 全部满足）
    pub violations: Vec<String>,
    /// 各项比例约束的超出量之和（比例单位，0.02 表示累计超出 2 个百分点）
    ///
    /// 供约束贝叶斯优化建模"违反程度"——仅有布尔可行标志时，
    /// 优化器无法感知"离可行边界还有多远"，在可行域稀疏时（实测
    /// 择优范围 1~500 时可行域仅约 19%）几乎必然收敛到不可行区。
    pub total_excess: f64,
    /// 缺供电量占负荷比例（比例单位）
    pub unserved_ratio: f64,
}

impl ConstraintStatus {
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }

    /// 违反量标量：0 表示全部满足，越大表示违反越严重
    pub fn violation_amount(&self) -> f64 {
        self.total_excess + self.unserved_ratio
    }
}

/// 评估 9 项约束中按全年指标校验的部分（逐时约束已在仿真中强制满足）
pub fn check_constraints(t: &Totals, tech_self_use_gen_min: f64, tech_self_use_load_min: f64, tech_feed_limit: f64, tech_curtail_limit: f64) -> ConstraintStatus {
    let total_gen = t.total_gen.max(1e-9);
    let load = t.load.max(1e-9);
    let gen_self_use = (t.total_gen - t.feed_in - t.curtailed).max(0.0); // 自发自用（含经储能）
    let load_green = (t.load - t.grid_import).max(0.0); // 负荷中绿电覆盖部分

    let self_use_gen_ratio = gen_self_use / total_gen * 100.0;
    let self_use_load_ratio = load_green / load * 100.0;
    let feed_ratio = t.feed_in / total_gen * 100.0;
    let curtail_ratio = t.curtailed / total_gen * 100.0;

    let mut violations = Vec::new();
    // 各项超出量（比例单位，累计用于约束贝叶斯优化的违反程度建模）
    let mut total_excess = 0.0f64;
    if self_use_gen_ratio + 1e-6 < tech_self_use_gen_min {
        total_excess += (tech_self_use_gen_min - self_use_gen_ratio) / 100.0;
        violations.push("自发自用占总可用发电量比例不足".to_string());
    }
    if self_use_load_ratio + 1e-6 < tech_self_use_load_min {
        total_excess += (tech_self_use_load_min - self_use_load_ratio) / 100.0;
        violations.push("自发自用占总用电量比例不足".to_string());
    }
    if feed_ratio - 1e-6 > tech_feed_limit {
        total_excess += (feed_ratio - tech_feed_limit) / 100.0;
        violations.push("余电上网比例超标".to_string());
    }
    if curtail_ratio - 1e-6 > tech_curtail_limit {
        total_excess += (curtail_ratio - tech_curtail_limit) / 100.0;
        violations.push("弃电率超标".to_string());
    }
    if t.unserved > 1e-6 {
        violations.push("存在缺供电量".to_string());
    }

    ConstraintStatus {
        self_use_gen_ratio,
        self_use_load_ratio,
        feed_ratio,
        curtail_ratio,
        violations,
        total_excess,
        unserved_ratio: t.unserved / load,
    }
}

/// 不可行方案展示罚值（固定哨兵值）
///
/// 依据：V3.0 实测敏感性分析表——不满足约束的方案适应度恒为 0.98
/// （如风电 +15%/+20%/+25% 三行因弃电率超标，适应度均为 0.98），
/// 与可行方案的 0.23x 量级形成明显区分。
pub const INFEASIBLE_FITNESS: f64 = 0.98;

/// 目标函数原始值（不含罚项）
pub fn objective_value(econ: &EconResult, objective: &str) -> f64 {
    match objective {
        "green" => econ.p_green,
        "capex" => econ.ic / 10_000.0, // 万元
        _ => econ.p_composite,         // composite
    }
}

/// 展示口径适应度：与 V3.0 敏感性分析表口径一致
///
/// - 满足全部约束 → 目标函数值（综合电价 / 绿电电价 / 初投资万元）
/// - 不满足约束   → 固定 0.98
pub fn fitness_visible(econ: &EconResult, cons: &ConstraintStatus, objective: &str) -> f64 {
    let base = objective_value(econ, objective);
    if !base.is_finite() || !cons.ok() {
        return INFEASIBLE_FITNESS;
    }
    base
}

/// 寻优口径适应度（目标函数 + 约束罚项），取值越小越优
///
/// 与展示口径的差异：不可行解不使用固定 0.98，而是叠加违反量，
/// 使优化器在不可行区内仍能识别"违反得多还是少"，避免采集函数
/// 在 0.98 常数平台上失去搜索方向（capex 目标下 0.98 甚至会小于
/// 可行解量级，导致不可行解被误判为更优）。
pub fn fitness_search(econ: &EconResult, cons: &ConstraintStatus, objective: &str) -> f64 {
    let base = objective_value(econ, objective);
    if !base.is_finite() {
        return f64::INFINITY;
    }
    if cons.ok() {
        return base;
    }

    // 罚值叠加在目标值之上（而非取常数平台），使不可行区仍保留目标函数的
    // 地形信息——代理模型需要在不可行区感知"往哪个方向走会变好"，
    // 若不可行区是常数平台，采集函数将失去搜索方向。
    //
    // 价格类目标量级 0.2~0.6 元/kWh，固定抬升 1.0 即足以严格劣于任何可行解；
    // 初投资目标量级可达数万元，固定抬升需按量级缩放，否则罚项可忽略不计。
    let scale = if objective == "capex" {
        base.abs().max(1.0)
    } else {
        1.0
    };
    let lift = 1.0 + cons.violation_amount();
    base + lift * scale
}

