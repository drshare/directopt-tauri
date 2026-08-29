//! 黄金基准对拍测试（与真实软件输出对比，确保计算逻辑正确）
//!
//! 基准文件：docs/20260829185838_curvetemplate_ldzl_3.0_16666555869.xlsx
//! （真实"新能源优化3.0"软件对同一输入（curvetemplate_ldzl_3.0.xlsx 曲线 +
//! inputtemplate_ldzl_3.0.xlsx 默认参数）的完整输出，含逐时 8760h 数据）。
//!
//! 输入夹具 golden_fixture.json 由该曲线模板逐时数据 + 前端默认参数生成。
//!
//! 对拍策略：
//! 1. 固定真实软件的最优配置，逐项对拍仿真/经济计算结果（验证计算逻辑正确性）；
//! 2. 遗传算法寻优结果不得劣于真实软件报告的最优目标值（验证寻优有效性；
//!    真实软件仅约 100 次评估，为启发式局部解，故不要求最优点完全一致）。

use std::sync::atomic::AtomicBool;

use super::economics::{check_constraints, compute_economics, fitness};
use super::engine::run_compute;
use super::params::{ComputeParams, CurveData};
use super::simulate::{Series, SimContext, Totals};

const FIXTURE: &str = include_str!("golden_fixture.json");

/// 真实软件最优配置（kW / kWh，取自基准文件 output Sheet 序号 1~4）
const REAL_WIND_KW: f64 = 192_725.010_474_436_4;
const REAL_PV_KW: f64 = 78_372.499_493_666_42;
const REAL_ESS_KWH: f64 = 45_477.247_181_967_09;
const REAL_ESS_KW: f64 = 22_738.623_590_983_55;
/// 真实软件最优目标值（评价周期内平均综合电价 元/kWh，序号 32）
const REAL_P_COMPOSITE: f64 = 0.235_335_892_912_168_9;

/// 数值完全一致允许的相对偏差（浮点尾差级别）
const EXACT_REL_TOL: f64 = 1e-9;

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

/// 在真实软件最优配置下执行与 engine 相同的评估链路
fn evaluate_at_real_optimum(f: &Fixture) -> (Totals, Series, super::economics::EconResult) {
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
    (totals, series.expect("series"), econ)
}

/// 逐时仿真累计量与真实软件合计行（output Sheet 第 8826 行，MWh）对拍
#[test]
fn golden_totals_match() {
    let f = fixture();
    let (t, _, _) = evaluate_at_real_optimum(&f);
    let m = |v: f64| v / 1000.0; // kWh → MWh

    assert_close(m(t.total_gen), 639_781.855_782_847_4, "全年新能源理论发电量");
    assert_close(t.grid_import, 154_321_732.313_134_62, "全年下网电量");
    assert_close(t.charge_ac, 16_463_796.521_910_57, "全年储能充电量(交流侧)");
    assert_close(t.charge_dc, 15_311_330.765_376_842, "全年储能实际充电量(直流侧)");
    assert_close(t.discharge_dc, 15_313_604.627_735_991, "全年储能实际放电量(直流侧)");
    assert_close(t.discharge_ac, 14_088_516.257_517_043, "全年储能供电量(交流侧)");
    assert_close(t.feed_in, 127_956_371.156_569_98, "全年余电上网电量");
    assert_close(t.curtailed, 103_131_936.675_022_44, "全年弃风弃光电量");
    assert_close(t.load, 560_640_000.0, "全年负荷用电量");
    assert_close(t.end_soc, 6_821.587_077_295_065_5, "储能年末剩余电量");
    assert_close(t.unserved, 0.0, "缺供电量");
}

/// 经济性指标与真实软件 output Sheet（序号 19~34，单位万元/元每kWh）对拍
#[test]
fn golden_economics_match() {
    let f = fixture();
    let (t, _, econ) = evaluate_at_real_optimum(&f);
    let w = |v: f64| v / 10_000.0; // 元 → 万元

    // 投资构成（序号 19~23）
    assert_close(w(f.params.econ.wind_invest * REAL_WIND_KW), 69_381.003_770_797_1, "风电系统投资");
    assert_close(w(f.params.econ.pv_invest * REAL_PV_KW), 21_160.574_863_289_934, "光伏系统投资");
    assert_close(w(f.params.econ.ess_invest * REAL_ESS_KWH), 2_728.634_830_918_025_3, "储能系统投资");
    assert_close(w(econ.ic), 94_770.213_465_005_07, "初投资");
    assert_close(w(econ.maint), 947.702_134_650_050_8, "运维成本");
    assert_close(w(econ.salary_cost), 100.0, "人员工资");

    // 年运行成本构成（序号 24~30）
    assert_close(w(econ.grid_buy_cost), 7_988.254_757_266_199, "年电网购电成本");
    assert_close(w(econ.self_use_cost), 1_203.437_443_599_306, "年自发自用输配成本");
    assert_close(w(econ.feed_revenue), 5_104.388_888_818_176, "年余电上网收益");
    assert_close(w(econ.annual_cost_display), 10_239.394_335_515_557, "年运行成本");
    assert_close(w(econ.replace_cost_pv), 1_436.006_355_064_888_8, "储能电池更换成本");

    // 全周期指标（序号 31~34）
    assert_close(w(econ.tc), 157_507.581_512_984_94, "评价周期内总成本");
    assert_close(econ.p_composite, 0.235_335_892_912_168_9, "综合电价");
    assert_close(econ.p_green, 0.128_116_729_099_992_55, "绿电电价");
    assert_close(econ.p_grid, 0.517_636_410_473_750_4, "网电电价");

    // 派生比例指标（由基准电量推得）
    let cons = check_constraints(
        &t,
        f.params.tech.self_use_gen_min,
        f.params.tech.self_use_load_min,
        f.params.tech.feed_limit,
        f.params.tech.curtail_limit,
    );
    let curtail_ratio = 103_131.936_675_022_44 / 639_781.855_782_847_4 * 100.0;
    assert_close(cons.curtail_ratio, curtail_ratio, "弃电率(内部一致性)");
    assert_close(cons.feed_ratio, 20.0, "余电上网比例");
    assert_close(cons.self_use_load_ratio, 72.474_006_079_991_68, "自发自用占总用电量比例");
    // 自发自用占总可用电量比例 = (理论发电量 − 余电上网 − 弃电) / 理论发电量 × 100
    // （基准值均取自真实软件输出：639781.86 − 127956.37 − 103131.94）/ 639781.86
    assert_close(
        cons.self_use_gen_ratio,
        (639_781.855_782_847_4 - 127_956.371_156_569_98 - 103_131.936_675_022_44)
            / 639_781.855_782_847_4
            * 100.0,
        "自发自用占总可用电量比例",
    );
}

/// 逐时曲线对拍：抽样小时（含首/末/峰谷）各列与真实软件逐时表一致
#[test]
fn golden_hourly_samples_match() {
    let f = fixture();
    let (_, s, _) = evaluate_at_real_optimum(&f);
    assert_eq!(s.wind.len(), 8760);

    // 真实软件逐时行（小时序号 1 基）→ 各列 kWh 值
    // 列序：风电, 光伏, 理论, 负荷, 实际, 充电AC, 充电DC, 弃电, 放电DC, 放电AC, 下网, 余电上网, 储能可用
    let samples: [(usize, [f64; 13]); 5] = [
        // 第 7 小时（2020-01-01 06:00）
        (
            6,
            [8459.423428512295, 0.0, 8459.423428512295, 64000.0, 8459.423428512295, 0.0, 0.0, 0.0, 0.0, 0.0, 55540.57657148771, 0.0, 6821.587077295066],
        ),
        // 第 14 小时（2020-01-01 13:00）
        (
            13,
            [0.0, 62051.42647411039, 62051.42647411039, 64000.0, 62051.42647411039, 0.0, 0.0, 0.0, 2118.014702053923, 1948.573525889609, 2.273736754432321e-13, 0.0, 9371.002136801666],
        ),
        // 第 100 小时（2020-01-05 03:00）
        (
            99,
            [69177.77524725182, 0.0, 69177.77524725182, 64000.0, 69177.77524725182, 5177.775247251819, 4815.330979944191, 0.0, 0.0, 0.0, 0.0, 0.0, 11636.91805723926],
        ),
        // 第 5000 小时（2020-07-28 07:00）
        (
            4999,
            [15627.63746809848, 7706.759737709688, 23334.39720580817, 64000.0, 23334.39720580817, 0.0, 0.0, 0.0, 24715.89520759081, 22738.62359098355, 17926.97920320828, 0.0, 20761.35197437628],
        ),
        // 第 8760 小时（2020-12-31 23:00）
        (
            8759,
            [43399.26329621216, 0.0, 43399.26329621216, 64000.0, 43399.26329621216, 0.0, 0.0, 0.0, 0.0, 0.0, 20600.73670378784, 0.0, 6821.587077295066],
        ),
    ];

    for (idx, row) in samples {
        let cols: [(&str, f64); 13] = [
            ("风电", s.wind[idx]),
            ("光伏", s.pv[idx]),
            ("理论发电", s.theory_gen[idx]),
            ("负荷", s.load[idx]),
            ("实际发电", s.actual_gen[idx]),
            ("充电AC", s.charge_ac[idx]),
            ("充电DC", s.charge_dc[idx]),
            ("弃电", s.curtailed[idx]),
            ("放电DC", s.discharge_dc[idx]),
            ("放电AC", s.discharge_ac[idx]),
            ("下网", s.grid_import[idx]),
            ("余电上网", s.feed_in[idx]),
            ("储能可用", s.soc_dc[idx]),
        ];
        for (col_idx, (name, actual)) in cols.iter().enumerate() {
            let expected = row[col_idx];
            let ok = (actual - expected).abs() <= 1e-6 || rel_diff(*actual, expected) <= EXACT_REL_TOL;
            assert!(
                ok,
                "第 {} 小时「{name}」与真实软件不一致: 实际={actual}, 期望={expected}",
                idx + 1
            );
        }
    }
}

/// 逐时合计（MWh）与真实软件合计行对拍
#[test]
fn golden_hourly_totals_match() {
    let f = fixture();
    let (_, s, _) = evaluate_at_real_optimum(&f);
    let sum = |v: &[f64]| v.iter().sum::<f64>() / 1000.0; // kWh → MWh

    let totals: [(&str, f64, f64); 13] = [
        ("风电发电量", sum(&s.wind), 493_774.221_114_093_03),
        ("光伏发电量", sum(&s.pv), 146_007.634_668_758_52),
        ("新能源理论发电量", sum(&s.theory_gen), 639_781.855_782_847_4),
        ("用户负荷", sum(&s.load), 560_640.0),
        ("新能源实际发电量", sum(&s.actual_gen), 536_649.919_107_828_4),
        ("储能充电量(交流侧)", sum(&s.charge_ac), 16_463.796_521_910_57),
        ("储能实际充电量(直流侧)", sum(&s.charge_dc), 15_311.330_765_376_842),
        ("弃风弃光电量", sum(&s.curtailed), 103_131.936_675_022_44),
        ("储能实际放电量(直流侧)", sum(&s.discharge_dc), 15_313.604_627_735_991),
        ("储能供电量(交流侧)", sum(&s.discharge_ac), 14_088.516_257_517_043),
        ("下网电量", sum(&s.grid_import), 154_321.732_313_134_62),
        ("余电上网电量", sum(&s.feed_in), 127_956.371_156_569_98),
        ("储能年末剩余电量", s.soc_dc[s.soc_dc.len() - 1] / 1000.0, 6.821_587_077_295_065_5),
    ];
    for (name, actual, expected) in totals {
        assert_close(actual, expected, &format!("逐时合计「{name}」"));
    }
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

    // 约束满足：弃电率 / 余电上网比例不超限
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

/// 全周期电量平衡恒等式在真实软件最优配置下成立（AR-2.1）
#[test]
fn golden_energy_balance_identity() {
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
    // 适应度口径：真实软件最优点的适应度 = 其报告的综合电价
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
    let cons = check_constraints(
        &t,
        f.params.tech.self_use_gen_min,
        f.params.tech.self_use_load_min,
        f.params.tech.feed_limit,
        f.params.tech.curtail_limit,
    );
    let fit = fitness(&econ, &cons, &t, "composite");
    assert_close(fit, REAL_P_COMPOSITE, "真实最优配置的适应度");
}
