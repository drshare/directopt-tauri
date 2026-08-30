//! 计算参数模型与后端权威校验（FR-5）
//! 前端校验仅做即时反馈，后端校验为准，确保计算输入准确。

use serde::Deserialize;

/// 技术参数（DR-1.1）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechParams {
    /// 储能充放电深度 %
    pub dod: f64,
    /// 电池充放电倍率 C
    pub rate: f64,
    /// 储能初始电量 %
    pub initial_soc: f64,
    /// 储能充电效率 %
    pub charge_eff: f64,
    /// 储能放电效率 %
    pub discharge_eff: f64,
    /// 接入公共电网容量（最大下网功率）kW
    pub grid_capacity: f64,
    /// 平均负荷率 %
    pub avg_load_rate: f64,
    /// 自发自用占总可用发电量比例下限 %
    pub self_use_gen_min: f64,
    /// 自发自用占总用电量比例下限 %
    pub self_use_load_min: f64,
    /// 余电上网比例上限 %
    pub feed_limit: f64,
    /// 余电最大上网功率 kW
    pub feed_power: f64,
    /// 弃电率上限 %
    pub curtail_limit: f64,
}

/// 经济评价参数（DR-1.2）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconParams {
    /// 风电系统单位投资 元/kW
    pub wind_invest: f64,
    /// 光伏系统单位投资 元/kW
    pub pv_invest: f64,
    /// 储能系统单位投资 元/kWh
    pub ess_invest: f64,
    /// 年运维费用占比 %
    pub opex_ratio: f64,
    /// 人员工资 万元/人年
    pub salary: f64,
    /// 定员人数 人
    pub staff_count: f64,
    /// 折现率 %
    pub discount_rate: f64,
    /// 评价周期 年
    pub eval_period: f64,
    /// 其他固定投资 万元
    pub other_invest: f64,
    /// 电池更换单价 元/kWh
    pub battery_replace_unit: f64,
    /// 电池更换比例 %
    pub battery_replace_ratio: f64,
    /// 电池更换时间 年末
    pub battery_replace_year: f64,
}

/// 遗传算法参数（AR-3.3，对应 V2.2 说明书口径）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GaParams {
    /// 遗传代数
    pub generations: u32,
    /// 交叉概率 0~1
    pub crossover_rate: f64,
    /// 变异概率 0~1
    pub mutation_rate: f64,
    /// 种群大小
    pub population_size: u32,
}

/// 贝叶斯优化参数（对应 V3.0 界面「算法参数」区）
///
/// 实测来源：http://150.158.94.206 绿电直连新能源优化配置 V3.0
/// 界面仅有「总评估次数」「初始随机采样点数」两项，V2.2 说明书中的
/// 遗传代数 / 交叉概率 / 变异概率 / 种群大小 在 V3.0 已不复存在。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoParams {
    /// 总评估次数（含初始采样点），V3.0 默认 100
    pub n_iter: u32,
    /// 初始随机采样点数，V3.0 默认 20
    pub n_init: u32,
}

impl Default for BoParams {
    fn default() -> Self {
        Self { n_iter: 100, n_init: 20 }
    }
}

/// 择优范围（AR-4：风电/光伏 MW，储能 MWh）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeParams {
    pub wind_start: f64,
    pub wind_end: f64,
    pub pv_start: f64,
    pub pv_end: f64,
    pub ess_start: f64,
    pub ess_end: f64,
}

/// 完整计算参数（由前端组装提交）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeParams {
    pub tech: TechParams,
    pub econ: EconParams,
    pub ga: GaParams,
    pub range: RangeParams,
    /// 寻优算法："bo"（贝叶斯优化，V3.0 口径，默认） | "ga"（遗传算法，V2.2 口径）
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    /// 贝叶斯优化参数（algorithm = "bo" 时生效）
    #[serde(default)]
    pub bo: BoParams,
    /// 输配电费方案："scheme1" | "scheme2"
    pub scheme: String,
    /// 优化目标："composite" | "green" | "capex"
    pub objective: String,
    /// 储能充电优先时段（0~23，空为不限）
    pub charge_periods: Vec<i32>,
    /// 储能允许放电时段（0~23，空为不限）
    pub discharge_periods: Vec<i32>,
}

/// 全年 8760h 输入曲线（曲线模板 curveldzl3 口径）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveData {
    /// 风电发电量标幺值
    pub wind_pu: Vec<f64>,
    /// 光伏发电量标幺值
    pub pv_pu: Vec<f64>,
    /// 用户负荷 kWh
    pub load: Vec<f64>,
    /// 电力现货市场交易电量电价 元/kWh
    pub price: Vec<f64>,
    /// 上网环节线损费用 元/kWh
    pub loss_fee: Vec<f64>,
    /// 电度输配电价 元/kWh
    pub tdu_fee: Vec<f64>,
    /// 系统运行费用 元/kWh
    pub system_fee: Vec<f64>,
    /// 政府性基金及附加 元/kWh
    pub fund_fee: Vec<f64>,
}

impl CurveData {
    /// 曲线长度一致性校验
    pub fn validate(&self) -> Result<(), String> {
        let n = self.load.len();
        if n == 0 {
            return Err("输入曲线为空，请上传有效的曲线模板文件".to_string());
        }
        let check = |name: &str, v: &[f64]| -> Result<(), String> {
            if v.len() != n {
                return Err(format!("曲线「{name}」长度（{}）与负荷曲线长度（{n}）不一致", v.len()));
            }
            if v.iter().any(|x| !x.is_finite()) {
                return Err(format!("曲线「{name}」存在非法数值（NaN/无穷）"));
            }
            Ok(())
        };
        check("风电出力标幺值", &self.wind_pu)?;
        check("光伏出力标幺值", &self.pv_pu)?;
        check("电量电价", &self.price)?;
        check("上网环节线损费", &self.loss_fee)?;
        check("电度输配电价", &self.tdu_fee)?;
        check("系统运行费", &self.system_fee)?;
        check("政府性基金及附加", &self.fund_fee)?;
        if self.load.iter().any(|x| *x < 0.0) {
            return Err("负荷曲线存在负值".to_string());
        }
        Ok(())
    }
}

fn check_pct(issues: &mut Vec<String>, label: &str, v: f64) {
    if !v.is_finite() {
        issues.push(format!("{label} 必须为数值"));
    } else if !(0.0..=100.0).contains(&v) {
        issues.push(format!("{label} 需在 0~100 之间"));
    }
}

fn check_range(issues: &mut Vec<String>, label: &str, s: f64, e: f64) {
    if s > e {
        issues.push(format!("{label}规模起始值不能大于结束值"));
    }
}

fn default_algorithm() -> String {
    "bo".to_string()
}

/// 后端权威参数校验（与前端 FR-5 规则一致并补充）
pub fn validate_params(p: &ComputeParams) -> Result<(), String> {
    let mut issues: Vec<String> = Vec::new();
    let t = &p.tech;

    for (label, v) in [
        ("储能充放电深度", t.dod),
        ("储能初始电量", t.initial_soc),
        ("储能充电效率", t.charge_eff),
        ("储能放电效率", t.discharge_eff),
        ("平均负荷率", t.avg_load_rate),
        ("自发自用占总可用发电量比例下限", t.self_use_gen_min),
        ("自发自用占总用电量比例下限", t.self_use_load_min),
        ("余电上网比例上限", t.feed_limit),
        ("弃电率上限", t.curtail_limit),
    ] {
        check_pct(&mut issues, label, v);
    }

    if t.dod + t.initial_soc < 100.0 {
        issues.push("储能初始电量 + 充放电深度 ≥ 100%".to_string());
    }
    // 按所选算法校验对应参数（另一套参数不参与校验）
    match p.algorithm.as_str() {
        "ga" => {
            if !(0.0..=1.0).contains(&p.ga.crossover_rate) {
                issues.push("交叉概率需在 0~1 之间".to_string());
            }
            if !(0.0..=1.0).contains(&p.ga.mutation_rate) {
                issues.push("变异概率需在 0~1 之间".to_string());
            }
            if p.ga.population_size < 4 {
                issues.push("种群大小不能小于 4".to_string());
            }
            if p.ga.generations < 1 {
                issues.push("遗传代数不能小于 1".to_string());
            }
        }
        _ => {
            // 贝叶斯优化（V3.0 默认）
            if p.bo.n_init < 2 {
                issues.push("初始随机采样点数不能小于 2".to_string());
            }
            if p.bo.n_iter <= p.bo.n_init {
                issues.push("总评估次数必须大于初始随机采样点数".to_string());
            }
        }
    }
    check_range(&mut issues, "风电", p.range.wind_start, p.range.wind_end);
    check_range(&mut issues, "光伏", p.range.pv_start, p.range.pv_end);
    check_range(&mut issues, "储能", p.range.ess_start, p.range.ess_end);
    if p.econ.eval_period < 1.0 {
        issues.push("评价周期不能小于 1 年".to_string());
    }
    if t.charge_eff <= 0.0 || t.discharge_eff <= 0.0 {
        issues.push("储能充/放电效率必须大于 0".to_string());
    }
    if t.rate <= 0.0 {
        issues.push("电池充放电倍率必须大于 0".to_string());
    }

    if !issues.is_empty() {
        return Err(format!(
            "参数校验未通过，共 {} 项：{}",
            issues.len(),
            issues.join("；")
        ));
    }
    Ok(())
}
