//! 8760h 逐时仿真引擎（AR-2 / AR-3.4）
//!
//! 调度策略（与标准模板 output Sheet 口径一致，已经真实软件输出对拍验证）：
//! - 风光出力 = 装机容量 × 出力标幺值（AR-2.2，恒等式）
//! - 盈余时段：储能充电（勾选时段内风光发电优先充电）→ 余电上网 → 弃电
//! - 余电上网分配：按电量电价从高到低逐时分配（受逐时上网功率与全年上网比例预算约束），
//!   预算耗尽或盈余全部上网后剩余部分弃电
//! - 缺口时段：储能放电（仅限允许放电时段）→ 下网购电（受并网容量约束）→ 缺供（记为重约束违反）
//! - 储能电量递推（AR-2.6）：E(i) = E(i-1) + 充电(交流)×ηc − 放电(交流)/ηd
//!   充电交流×ηc 为直流侧入电，放电交流/ηd 为直流侧出电（与模板推算口径一致）

use crate::compute::params::{CurveData, TechParams};

/// 单方案全年仿真累计量
#[derive(Debug, Clone, Default)]
pub struct Totals {
    /// 新能源理论发电量 kWh
    pub total_gen: f64,
    /// 弃风弃光电量 kWh
    pub curtailed: f64,
    /// 余电上网电量 kWh
    pub feed_in: f64,
    /// 下网电量 kWh
    pub grid_import: f64,
    /// 储能充电量（交流侧）kWh
    pub charge_ac: f64,
    /// 储能对外供电量（交流侧）kWh
    pub discharge_ac: f64,
    /// 储能实际充电量（直流侧）kWh
    pub charge_dc: f64,
    /// 储能实际放电量（直流侧）kWh
    pub discharge_dc: f64,
    /// 负荷用电量 kWh
    pub load: f64,
    /// 缺供电量 kWh（正常应为 0）
    pub unserved: f64,
    /// 储能年末剩余电量（直流侧）kWh
    pub end_soc: f64,
    /// Σ 下网电量×(电量电价+线损+系统运行费+基金附加) 元
    pub grid_energy_cost: f64,
    /// Σ 下网电量×电度输配电价 元（方案二购电输配部分）
    pub grid_tdu_cost: f64,
    /// Σ 新能源实际发电量×政府性基金及附加 元（方案一自发自用输配成本，与真实软件口径一致）
    pub self_fund_cost: f64,
    /// Σ 新能源实际发电量×(输配电价+政府性基金及附加) 元（方案二自发自用输配成本）
    pub self_tdu_fund_cost: f64,
    /// Σ 余电上网电量×电量电价 元（余电上网收益）
    pub feed_revenue: f64,
    /// 电度输配电价全年均值 元/kWh（方案一容量法输配电费用）
    pub avg_tdu: f64,
}

/// 逐时序列（仅最优方案存储，供结果展示与报告导出）
#[derive(Debug, Clone, Default)]
pub struct Series {
    pub wind: Vec<f64>,
    pub pv: Vec<f64>,
    pub theory_gen: Vec<f64>,
    pub load: Vec<f64>,
    pub actual_gen: Vec<f64>,
    pub charge_ac: Vec<f64>,
    pub charge_dc: Vec<f64>,
    pub curtailed: Vec<f64>,
    pub discharge_dc: Vec<f64>,
    pub discharge_ac: Vec<f64>,
    pub grid_import: Vec<f64>,
    pub feed_in: Vec<f64>,
    pub soc_dc: Vec<f64>,
}

/// 单方案仿真上下文（由装机与效率参数派生，避免每次迭代重复计算）
pub struct SimContext<'a> {
    pub wind_kw: f64,
    pub pv_kw: f64,
    pub ess_kw: f64,
    pub soc_min: f64,
    pub soc_max: f64,
    pub soc_init: f64,
    pub eff_c: f64,
    pub eff_d: f64,
    pub grid_capacity: f64,
    pub feed_power: f64,
    /// 全年余电上网电量预算 kWh（feed_limit × 理论发电量，与调度无关可预计算）
    pub feed_budget: f64,
    charge_hours: [bool; 24],
    discharge_hours: [bool; 24],
    /// 用户显式勾选了充电时段：时段内风光发电优先给储能充电（DR-1.1）
    charge_priority: bool,
    curves: &'a CurveData,
}

impl<'a> SimContext<'a> {
    pub fn new(
        wind_kw: f64,
        pv_kw: f64,
        ess_kwh: f64,
        tech: &TechParams,
        charge_periods: &[i32],
        discharge_periods: &[i32],
        curves: &'a CurveData,
    ) -> Self {
        let eff_c = tech.charge_eff / 100.0;
        let eff_d = tech.discharge_eff / 100.0;
        let soc_max = ess_kwh;
        let soc_min = ess_kwh * (1.0 - tech.dod / 100.0).clamp(0.0, 1.0);
        let soc_init = (ess_kwh * tech.initial_soc / 100.0).clamp(soc_min, soc_max);

        // 理论发电量与调度无关，可预计算（用于全年上网电量预算）
        let mut total_gen = 0.0;
        for i in 0..curves.load.len() {
            total_gen += wind_kw * curves.wind_pu[i] + pv_kw * curves.pv_pu[i];
        }

        let mut ch = [false; 24];
        if charge_periods.is_empty() {
            ch.fill(true);
        } else {
            for &h in charge_periods {
                if (0..24).contains(&h) {
                    ch[h as usize] = true;
                }
            }
        }
        let mut dh = [false; 24];
        if discharge_periods.is_empty() {
            dh.fill(true);
        } else {
            for &h in discharge_periods {
                if (0..24).contains(&h) {
                    dh[h as usize] = true;
                }
            }
        }

        Self {
            wind_kw,
            pv_kw,
            ess_kw: ess_kwh * tech.rate,
            soc_min,
            soc_max,
            soc_init,
            eff_c,
            eff_d,
            grid_capacity: tech.grid_capacity,
            feed_power: tech.feed_power,
            feed_budget: tech.feed_limit / 100.0 * total_gen,
            charge_hours: ch,
            discharge_hours: dh,
            charge_priority: !charge_periods.is_empty(),
            curves,
        }
    }

    /// 执行全年逐时仿真，返回累计量；`store_series` 为 true 时同时返回逐时序列
    pub fn run(&self, store_series: bool) -> (Totals, Option<Series>) {
        let c = self.curves;
        let n = c.load.len();
        let mut t = Totals::default();
        t.avg_tdu = if n > 0 { c.tdu_fee.iter().sum::<f64>() / n as f64 } else { 0.0 };

        let mut series = if store_series {
            Some(Series::default())
        } else {
            None
        };

        let mut soc = self.soc_init;

        // ---- 第一遍：储能充放电与电网调度（与余电上网分配解耦）----
        let mut surplus = vec![0.0f64; n]; // 各小时可上网盈余 kWh
        for i in 0..n {
            let hour = i % 24;
            let wind_i = self.wind_kw * c.wind_pu[i];
            let pv_i = self.pv_kw * c.pv_pu[i];
            let theory = wind_i + pv_i;
            let load_i = c.load[i];

            let mut charge_ac = 0.0f64;
            let mut discharge_ac = 0.0f64;
            let mut grid_i = 0.0f64;

            // ---- 储能充电（AR-2.4 / AR-2.5 / AR-2.6）----
            if self.charge_hours[hour] && soc < self.soc_max {
                let headroom_dc = (self.soc_max - soc).max(0.0);
                let max_ac = (headroom_dc / self.eff_c).min(self.ess_kw);
                let surplus_now = (theory - load_i).max(0.0);
                if surplus_now > 0.0 {
                    // 盈余充电：充盈余部分
                    charge_ac = max_ac.min(surplus_now);
                } else if self.charge_priority {
                    // 显式勾选充电时段：风光发电优先给储能充电，有余电再供负荷
                    charge_ac = max_ac.min(theory);
                }
            }

            let net = theory - charge_ac - load_i;

            if net >= 0.0 {
                // 盈余：可上网电量（上网/弃电分配在第二遍按电价优先级统一进行）
                surplus[i] = net;
            } else {
                // ---- 缺口：储能放电（AR-2.4/2.5）→ 下网购电（AR-2.3）→ 缺供 ----
                let mut deficit = -net;
                if self.discharge_hours[hour] && soc > self.soc_min {
                    let avail_dc = (soc - self.soc_min).max(0.0);
                    let max_ac = (avail_dc * self.eff_d).min(self.ess_kw);
                    discharge_ac = max_ac.min(deficit);
                    deficit -= discharge_ac;
                }
                grid_i = deficit.min(self.grid_capacity);
                t.unserved += deficit - grid_i;
            }

            // ---- 储能电量递推（AR-2.6），并钳制在容量约束（AR-2.5）内 ----
            soc += charge_ac * self.eff_c - discharge_ac / self.eff_d;
            soc = soc.clamp(self.soc_min, self.soc_max);

            // ---- 逐时累计（与上网分配无关的部分）----
            t.total_gen += theory;
            t.grid_import += grid_i;
            t.charge_ac += charge_ac;
            t.discharge_ac += discharge_ac;
            t.charge_dc += charge_ac * self.eff_c;
            t.discharge_dc += discharge_ac / self.eff_d;
            t.load += load_i;

            let full_price = c.price[i] + c.loss_fee[i] + c.system_fee[i] + c.fund_fee[i];
            t.grid_energy_cost += grid_i * full_price;
            t.grid_tdu_cost += grid_i * c.tdu_fee[i];

            if let Some(s) = series.as_mut() {
                s.wind.push(wind_i);
                s.pv.push(pv_i);
                s.theory_gen.push(theory);
                s.load.push(load_i);
                s.charge_ac.push(charge_ac);
                s.charge_dc.push(charge_ac * self.eff_c);
                s.discharge_dc.push(discharge_ac / self.eff_d);
                s.discharge_ac.push(discharge_ac);
                s.grid_import.push(grid_i);
                s.soc_dc.push(soc);
            }
        }

        // ---- 第二遍：余电上网分配（与真实软件口径一致，已对拍验证）----
        // 按电量电价从高到低逐时分配上网电量：
        // feed_i = min(盈余_i, 余电最大上网功率, 全年上网预算剩余)，其余弃电。
        let mut order: Vec<usize> = (0..n).filter(|&i| surplus[i] > 0.0).collect();
        order.sort_by(|&a, &b| {
            c.price[b]
                .partial_cmp(&c.price[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut feed = vec![0.0f64; n];
        let mut budget_left = self.feed_budget;
        for &i in &order {
            if budget_left <= 0.0 {
                break;
            }
            let feed_i = surplus[i].min(self.feed_power).min(budget_left);
            feed[i] = feed_i;
            budget_left -= feed_i;
        }

        // 全量遍历：累计上网/弃电相关指标并填充逐时序列
        for i in 0..n {
            let theory_i = self.wind_kw * c.wind_pu[i] + self.pv_kw * c.pv_pu[i];
            let curtail_i = surplus[i] - feed[i];
            let actual_gen_i = theory_i - curtail_i;
            t.feed_in += feed[i];
            t.curtailed += curtail_i;
            t.feed_revenue += feed[i] * c.price[i];
            // 自发自用输配成本基数 = 新能源实际发电量（真实软件口径）
            t.self_fund_cost += actual_gen_i * c.fund_fee[i];
            t.self_tdu_fund_cost += actual_gen_i * (c.tdu_fee[i] + c.fund_fee[i]);

            if let Some(s) = series.as_mut() {
                s.actual_gen.push(actual_gen_i);
                s.curtailed.push(curtail_i);
                s.feed_in.push(feed[i]);
            }
        }

        t.end_soc = soc;
        (t, series)
    }
}
