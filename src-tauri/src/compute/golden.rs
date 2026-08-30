//! 黄金基准对拍测试（与真实软件输出对比，确保计算逻辑正确）
//!
//! 基准文件：docs/20260829185838_curvetemplate_ldzl_3.0_16666555869.xlsx
//! （真实"新能源优化3.0"软件对同一输入（curvetemplate_ldzl_3.0.xlsx 曲线 +
//! inputtemplate_ldzl_3.0.xlsx 默认参数）的完整输出）。
//!
//! 基准夹具（由该文件提取，随测试固化）：
//! - golden_fixture.json  — 输入曲线（curveldzl3 全年 8760h × 8 列）+ 前端默认参数
//! - golden_hourly.json   — 真实软件逐时电量平衡表（output Sheet 8760 行 × 13 列）
//! - golden_expected.json — 真实软件输出指标全表（output Sheet 序号 1~34）
//!
//! 对拍维度（逐级细化）：
//! 1. 逐时全量：8760h × 13 列逐位对拍 + 逐列偏差统计报告；
//! 2. 指标全表：34 项输出指标按标签逐一对应（含别名映射），浮点精度级对拍；
//! 3. 仿真累计量 / 经济中间量对拍；
//! 4. 寻优有效性：GA 结果不劣于真实软件报告值（真实软件仅约 100 次评估，
//!    为启发式局部解，故不要求最优点一致，只要求计算逻辑与寻优质量）。

use std::sync::atomic::AtomicBool;

use super::economics::{compute_economics, fitness_search};
use super::engine::{build_payload, run_compute};
use super::params::{ComputeParams, CurveData};
use super::simulate::SimContext;

const FIXTURE: &str = include_str!("golden_fixture.json");
const GOLDEN_HOURLY: &str = include_str!("golden_hourly.json");
const GOLDEN_EXPECTED: &str = include_str!("golden_expected.json");

/// 真实软件最优配置（kW / kWh，取自基准文件 output Sheet 序号 1~4）
const REAL_WIND_KW: f64 = 192_725.010_474_436_4;
const REAL_PV_KW: f64 = 78_372.499_493_666_42;
const REAL_ESS_KWH: f64 = 45_477.247_181_967_09;
/// 真实软件最优目标值（评价周期内平均综合电价 元/kWh，序号 32）
const REAL_P_COMPOSITE: f64 = 0.235_335_892_912_168_9;

/// 数值完全一致允许的相对偏差（浮点尾差级别）
const EXACT_REL_TOL: f64 = 1e-9;
/// 逐时数值允许的绝对偏差（真实文件中存在 2.27e-13 量级的浮点残差）
const HOURLY_ABS_TOL: f64 = 1e-6;

struct Fixture {
    params: ComputeParams,
    curves: CurveData,
}

fn fixture() -> Fixture {
    let v: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture json");
    let params: ComputeParams = serde_json::from_value(v["params"].clone()).expect("params");
    let curves: CurveData = serde_json::from_value(v["curves"].clone()).expect("curves");
    Fixture { params, curves }
}

/// 真实软件逐时基准表（8760 行 × 13 列）
struct GoldenHourly {
    columns: Vec<String>,
    rows: Vec<Vec<f64>>,
}

fn golden_hourly() -> GoldenHourly {
    let v: serde_json::Value = serde_json::from_str(GOLDEN_HOURLY).expect("hourly json");
    let columns = v["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .map(|c| c.as_str().expect("col name").to_string())
        .collect();
    let rows: Vec<Vec<f64>> = v["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| {
            r.as_array()
                .expect("row")
                .iter()
                .map(|x| x.as_f64().expect("num"))
                .collect::<Vec<f64>>()
        })
        .collect();
    assert_eq!(rows.len(), 8760, "逐时基准应为 8760 行");
    GoldenHourly { columns, rows }
}

/// 真实软件输出指标全表（34 项：label → value）
fn golden_metrics() -> Vec<(String, f64)> {
    let v: Vec<serde_json::Value> = serde_json::from_str(GOLDEN_EXPECTED).expect("expected json");
    v.iter()
        .map(|m| {
            (
                m["label"].as_str().expect("label").to_string(),
                m["value"].as_f64().expect("value"),
            )
        })
        .collect()
}

fn rel_diff(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1e-12)
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    let diff = rel_diff(actual, expected);
    assert!(
        diff <= EXACT_REL_TOL,
        "{what} 与真实软件不一致: 实际={actual}, 期望={expected}, 相对偏差={diff:.3e}"
    );
}

/// 在真实软件最优配置下走完整评估链路（仿真 → 经济 → 结果负载），
/// 与 run_compute 寻优后的产出路径共用同一 build_payload，保证口径一致。
fn evaluate_real_optimum(f: &Fixture) -> super::engine::ComputeResultPayload {
    let ctx = SimContext::new(
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        &f.params.tech,
        &f.params.charge_periods,
        &f.params.discharge_periods,
        &f.curves,
    );
    let (totals, series) = ctx.run(true);
    let series = series.expect("series");
    let econ = compute_economics(
        &totals,
        &f.params.econ,
        &f.params.scheme,
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        f.params.tech.grid_capacity,
        f.params.tech.avg_load_rate,
    );
    build_payload(
        &f.params,
        &f.curves,
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        &totals,
        &series,
        &econ,
        f64::NAN,
    )
}

/// 输入夹具与真实曲线模板的关键位对拍（保证夹具忠实于输入文件）
#[test]
fn golden_input_curves_match_template() {
    let f = fixture();
    let c = &f.curves;
    for s in [
        &c.load,
        &c.wind_pu,
        &c.pv_pu,
        &c.price,
        &c.loss_fee,
        &c.tdu_fee,
        &c.system_fee,
        &c.fund_fee,
    ] {
        assert_eq!(s.len(), 8760, "曲线长度应为 8760");
    }
    assert_eq!(c.load[0], 64_000.0);
    assert!((c.wind_pu[6] - 0.043_893_75).abs() < 1e-12);
    assert!((c.pv_pu[9] - 0.207_31).abs() < 1e-12);
    assert!((c.price[0] - 0.146_592).abs() < 1e-12);
    assert!((c.price[9] - 0.305_4).abs() < 1e-12);
    assert!((c.fund_fee[0] - 0.022_425).abs() < 1e-12);
    // 择优范围与 GA 参数与前端默认一致
    assert_eq!(f.params.ga.population_size, 100);
    assert_eq!(f.params.ga.generations, 40);
}

/// 【细化 1】逐时全量对拍：8760h × 13 列
///
/// 判据分层：
/// - 与上网分配无关的 10 列（风光/理论/负荷/充放电/下网/储能可用）：逐小时精确对拍；
/// - 受上网分配影响的三列（实际发电量 / 弃电 / 余电上网）：
///   * 非边界电价档（整档全上网或整档全弃电）：逐小时精确对拍；
///   * 预算边界档（档内部分上网、部分弃电）：真实软件在**同价小时之间的取舍**
///     是其排序实现细节（不稳定排序，不影响任何经济量），按**档位总量守恒**对拍；
/// - 全年余电上网收益（经济决定量）：逐位精确对拍。
#[test]
fn golden_hourly_full_match() {
    let f = fixture();
    let payload = evaluate_real_optimum(&f);
    let b = &payload.balance;
    let golden = golden_hourly();
    let price = &f.curves.price;

    // 列序与基准文件 output Sheet D~P 列一致
    let ours: Vec<&Vec<f64>> = vec![
        &b.wind,
        &b.pv,
        &b.theory_gen,
        &b.load,
        &b.actual_gen,
        &b.charge_ac,
        &b.charge_dc,
        &b.curtailed,
        &b.discharge_dc,
        &b.discharge_ac,
        &b.grid_import,
        &b.feed_in,
        &b.soc_dc,
    ];
    assert_eq!(ours.len(), golden.columns.len(), "列数应与基准一致");

    // ---- 第一部分：与上网分配无关的 10 列逐小时精确对拍 ----
    // （除 actual(4) / curt(7) / feed(11) 三列，见第二部分分层对拍）
    let dispatch_cols = [0usize, 1, 2, 3, 5, 6, 8, 9, 10, 12];
    println!(
        "{:<12} {:>14} {:>14} {:>12} {:>10}",
        "列", "最大绝对偏差", "最大相对偏差", "不一致数", "总计(MWh)"
    );
    for &col_idx in &dispatch_cols {
        let name = &golden.columns[col_idx];
        let ours_col = ours[col_idx];
        let mut max_abs = 0.0f64;
        let mut max_rel = 0.0f64;
        let mut mismatches = 0usize;
        for (h, expected) in golden.rows.iter().map(|r| r[col_idx]).enumerate() {
            let actual = ours_col[h];
            let abs = (actual - expected).abs();
            max_abs = max_abs.max(abs);
            if expected.abs() > 1e-9 {
                max_rel = max_rel.max(rel_diff(actual, expected));
            }
            if abs > HOURLY_ABS_TOL && rel_diff(actual, expected) > EXACT_REL_TOL {
                mismatches += 1;
            }
        }
        let sum_mwh: f64 = ours_col.iter().sum::<f64>() / 1000.0;
        println!(
            "{:<12} {:>14.3e} {:>14.3e} {:>12} {:>10.2}",
            name, max_abs, max_rel, mismatches, sum_mwh
        );
        assert_eq!(
            mismatches, 0,
            "逐时列「{name}」有 {mismatches} 小时与真实软件不一致"
        );
    }

    // ---- 第二部分：上网分配相关列（actual/curt/feed）按电价档分层对拍 ----
    // 按电价分档（f64 精确相等即可，价格重复值完全一致）
    let mut tiers: Vec<(f64, Vec<usize>)> = Vec::new();
    for (h, &p) in price.iter().enumerate() {
        match tiers.iter_mut().find(|(tp, _)| *tp == p) {
            Some((_, hours)) => hours.push(h),
            None => tiers.push((p, vec![h])),
        }
    }

    let feed_ours = &b.feed_in;
    let curt_ours = &b.curtailed;
    let actual_ours = &b.actual_gen;
    let feed_real = golden.rows.iter().map(|r| r[11]).collect::<Vec<_>>();
    let curt_real = golden.rows.iter().map(|r| r[7]).collect::<Vec<_>>();
    let actual_real = golden.rows.iter().map(|r| r[4]).collect::<Vec<_>>();

    let mut boundary_tiers = 0usize;
    let mut per_hour_checked = 0usize;
    for &(p, ref hours) in &tiers {
        // 档位总量守恒（任何同价内部分配方式都必须满足）
        for (name, o_col, r_col) in [
            ("余电上网", feed_ours, &feed_real),
            ("弃电", curt_ours, &curt_real),
            ("实际发电量", actual_ours, &actual_real),
        ] {
            let so: f64 = hours.iter().map(|&h| o_col[h]).sum();
            let sr: f64 = hours.iter().map(|&h| r_col[h]).sum();
            assert_close(so, sr, &format!("电价 {p} 档「{name}」档内总量"));
        }

        // 判断该档是否为预算边界档：
        // - 档内上网总量 = 0 → 整档弃电（确定性）；
        // - 档内上网总量 = Σ min(盈余, 上网功率上限) → 受功率上限决定（确定性）；
        // - 其余（年度电量预算恰在本档内耗尽）→ 同价小时取舍取决于真实软件排序实现，
        //   仅对拍档位总量（经济量不受影响）。
        let surplus_at = |h: usize| {
            (b.theory_gen[h] - b.charge_ac[h] - b.load[h]).max(0.0)
        };
        let tier_cap_total: f64 = hours
            .iter()
            .map(|&h| surplus_at(h).min(f.params.tech.feed_power))
            .sum();
        let tier_feed_real: f64 = hours.iter().map(|&h| feed_real[h]).sum();
        let budget_limited = tier_feed_real > HOURLY_ABS_TOL
            && (tier_feed_real - tier_cap_total).abs() > HOURLY_ABS_TOL;
        if budget_limited {
            boundary_tiers += 1;
            continue; // 边界档内同价小时取舍属于实现细节，仅对拍档位总量
        }

        // 非边界档：逐小时精确对拍
        for &h in hours {
            for (name, o_col, r_col) in [
                ("余电上网", feed_ours, &feed_real),
                ("弃电", curt_ours, &curt_real),
                ("实际发电量", actual_ours, &actual_real),
            ] {
                let diff = (o_col[h] - r_col[h]).abs();
                assert!(
                    diff <= HOURLY_ABS_TOL || rel_diff(o_col[h], r_col[h]) <= EXACT_REL_TOL,
                    "第 {} 小时（电价 {p}）「{name}」不一致: 实际={}, 期望={}",
                    h + 1,
                    o_col[h],
                    r_col[h]
                );
            }
            per_hour_checked += 1;
        }
    }

    // ---- 第三部分：全年余电上网收益（经济决定量）逐位对拍 ----
    let revenue_ours: f64 = b.feed_in.iter().zip(price.iter()).map(|(f, &p)| f * p).sum();
    let revenue_real: f64 = feed_real.iter().zip(price.iter()).map(|(f, &p)| f * p).sum();
    assert_close(revenue_ours / 10_000.0, 5_104.388_888_818_176, "全年余电上网收益");
    assert_close(revenue_ours, revenue_real, "全年余电上网收益(与真实软件对比)");

    println!(
        "上网分配分层对拍：非边界档逐小时 {per_hour_checked} 小时全部一致；预算边界档 {boundary_tiers} 个（档位总量守恒）；全年收益一致"
    );
}

/// 【细化 2】输出指标全表对拍：真实软件 34 项按标签逐一对应（浮点精度级）
#[test]
fn golden_output_metrics_match() {
    let f = fixture();
    let payload = evaluate_real_optimum(&f);

    // 真实软件标签 → 本系统指标标签（其余同名）
    let alias: &[(&str, &str)] = &[
        ("评价周期内总成本", "周期内总成本"),
        ("评价周期内平均综合电价", "综合电价"),
        ("评价周期内平均绿电电价", "绿电电价"),
        ("评价周期内平均网电电价", "网电电价"),
    ];
    let find = |label: &str| -> Option<f64> {
        payload
            .headline
            .iter()
            .chain(payload.invest.iter())
            .chain(payload.opex.iter())
            .chain(payload.energy_stats.iter())
            .find(|m| m.label == label)
            .map(|m| m.value)
    };

    let mut checked = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    for (real_label, expected) in golden_metrics() {
        let ours_label = alias
            .iter()
            .find(|(from, _)| *from == real_label)
            .map(|(_, to)| to.to_string())
            .unwrap_or_else(|| real_label.clone());
        match find(&ours_label) {
            Some(actual) => {
                assert_close(actual, expected, &format!("指标「{real_label}」"));
                checked += 1;
            }
            None => skipped.push(real_label),
        }
    }
    println!("指标全表对拍：{checked}/34 项逐一比对通过，无对应项: {skipped:?}");
    // 真实软件 34 项中除输出文件特有的公式中间项外应全部可对应
    assert!(checked >= 30, "可对拍指标过少: {checked}");
}

/// 仿真累计量与真实软件合计行（output Sheet 第 8826 行，MWh）对拍
#[test]
fn golden_totals_match() {
    let f = fixture();
    let payload = evaluate_real_optimum(&f);
    let b = &payload.balance;
    let m = |v: f64| v / 1000.0; // kWh → MWh

    assert_close(m(b.theory_gen.iter().sum()), 639_781.855_782_847_4, "全年新能源理论发电量");
    assert_close(b.grid_import.iter().sum(), 154_321_732.313_134_62, "全年下网电量");
    assert_close(b.charge_ac.iter().sum(), 16_463_796.521_910_57, "全年储能充电量(交流侧)");
    assert_close(b.charge_dc.iter().sum(), 15_311_330.765_376_842, "全年储能实际充电量(直流侧)");
    assert_close(b.discharge_dc.iter().sum(), 15_313_604.627_735_991, "全年储能实际放电量(直流侧)");
    assert_close(b.discharge_ac.iter().sum(), 14_088_516.257_517_043, "全年储能供电量(交流侧)");
    assert_close(b.feed_in.iter().sum(), 127_956_371.156_569_98, "全年余电上网电量");
    assert_close(b.curtailed.iter().sum(), 103_131_936.675_022_44, "全年弃风弃光电量");
    assert_close(b.load.iter().sum(), 560_640_000.0, "全年负荷用电量");
    assert_close(b.soc_dc[b.soc_dc.len() - 1], 6_821.587_077_295_065_5, "储能年末剩余电量");
}

/// 经济中间量与真实软件对拍（元口径，经 evaluate_real_optimum 全链路）
#[test]
fn golden_economics_match() {
    let f = fixture();
    let ctx = SimContext::new(
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        &f.params.tech,
        &f.params.charge_periods,
        &f.params.discharge_periods,
        &f.curves,
    );
    let (totals, _) = ctx.run(false);
    let econ = compute_economics(
        &totals,
        &f.params.econ,
        &f.params.scheme,
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        f.params.tech.grid_capacity,
        f.params.tech.avg_load_rate,
    );
    let w = |v: f64| v / 10_000.0; // 元 → 万元

    assert_close(w(econ.ic), 94_770.213_465_005_07, "初投资");
    assert_close(w(econ.maint), 947.702_134_650_050_8, "运维成本");
    assert_close(w(econ.salary_cost), 100.0, "人员工资");
    assert_close(w(econ.grid_buy_cost), 7_988.254_757_266_199, "年电网购电成本");
    assert_close(w(econ.self_use_cost), 1_203.437_443_599_306, "年自发自用输配成本");
    assert_close(w(econ.feed_revenue), 5_104.388_888_818_176, "年余电上网收益");
    assert_close(w(econ.annual_cost_display), 10_239.394_335_515_557, "年运行成本");
    assert_close(w(econ.replace_cost_pv), 1_436.006_355_064_888_8, "储能电池更换成本");
    assert_close(w(econ.tc), 157_507.581_512_984_94, "评价周期内总成本");
    assert_close(econ.p_composite, 0.235_335_892_912_168_9, "综合电价");
    assert_close(econ.p_green, 0.128_116_729_099_992_55, "绿电电价");
    assert_close(econ.p_grid, 0.517_636_410_473_750_4, "网电电价");
}

/// 遗传算法寻优：结果不得劣于真实软件报告的最优目标值，且满足全部约束
#[test]
fn golden_optimize_at_least_as_good_as_real_software() {
    let f = fixture();
    let curtail_limit = f.params.tech.curtail_limit;
    let on_progress = |_: f64, _: String| {};
    let cancelled = AtomicBool::new(false);
    let payload = run_compute(f.params, f.curves, &on_progress, &cancelled).expect("run_compute ok");

    assert!(
        payload.best.fitness <= REAL_P_COMPOSITE + 1e-9,
        "遗传算法最优目标值({})劣于真实软件结果({REAL_P_COMPOSITE})",
        payload.best.fitness
    );

    let curtail = payload
        .headline
        .iter()
        .find(|m| m.label == "弃电率")
        .expect("弃电率")
        .value;
    assert!(curtail <= curtail_limit + 1e-6, "弃电率 {curtail}% 超限");

    // 内部一致性：初投资 = 各系统投资之和
    let get = |label: &str| {
        payload
            .headline
            .iter()
            .chain(payload.invest.iter())
            .find(|m| m.label == label)
            .unwrap_or_else(|| panic!("缺少指标「{label}」"))
            .value
    };
    let ic_sum = get("风电系统投资") + get("光伏系统投资") + get("储能系统投资") + get("其他固定投资");
    assert!((get("初投资") - ic_sum).abs() / ic_sum < 1e-9, "初投资与分项之和不一致");

    // 综合电价 = 最优适应度（objective = composite）
    assert!(
        (get("综合电价") - payload.best.fitness).abs() < 1e-9,
        "综合电价与最优适应度不一致"
    );
}

/// 全周期电量平衡恒等式在真实软件最优配置下成立（AR-2.1），且适应度口径一致
#[test]
fn golden_energy_balance_and_fitness() {
    let f = fixture();
    let ctx = SimContext::new(
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        &f.params.tech,
        &f.params.charge_periods,
        &f.params.discharge_periods,
        &f.curves,
    );
    let (t, _) = ctx.run(false);
    let lhs = t.load + t.charge_ac + t.feed_in;
    let rhs = (t.total_gen - t.curtailed) + t.discharge_ac + t.grid_import + t.unserved;
    assert!(
        (lhs - rhs).abs() / lhs.max(1.0) < 1e-9,
        "电量平衡不成立: lhs={lhs}, rhs={rhs}"
    );
    let econ = compute_economics(
        &t,
        &f.params.econ,
        &f.params.scheme,
        REAL_WIND_KW,
        REAL_PV_KW,
        REAL_ESS_KWH,
        f.params.tech.grid_capacity,
        f.params.tech.avg_load_rate,
    );
    let cons = super::economics::check_constraints(
        &t,
        f.params.tech.self_use_gen_min,
        f.params.tech.self_use_load_min,
        f.params.tech.feed_limit,
        f.params.tech.curtail_limit,
    );
    let fit = fitness_search(&econ, &cons, "composite");
    assert_close(fit, REAL_P_COMPOSITE, "真实最优配置的适应度");
}
