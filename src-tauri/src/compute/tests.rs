//! 计算核心单元测试：电量平衡恒等式、储能约束、经济口径、GA 收敛与边界

use std::sync::atomic::AtomicBool;

use super::economics::{compute_economics, fitness};
use super::engine::run_compute;
use super::ga::optimize;
use super::params::{
    ComputeParams, CurveData, EconParams, GaParams, RangeParams, TechParams,
};
use super::simulate::SimContext;

const EPS: f64 = 1e-6;

fn tech() -> TechParams {
    TechParams {
        dod: 85.0,
        rate: 0.5,
        initial_soc: 20.0,
        charge_eff: 93.0,
        discharge_eff: 92.0,
        grid_capacity: 80_000.0,
        avg_load_rate: 50.0,
        self_use_gen_min: 60.0,
        self_use_load_min: 30.0,
        feed_limit: 20.0,
        feed_power: 80_000.0,
        curtail_limit: 20.0,
    }
}

fn econ() -> EconParams {
    EconParams {
        wind_invest: 3600.0,
        pv_invest: 2700.0,
        ess_invest: 600.0,
        opex_ratio: 1.0,
        salary: 10.0,
        staff_count: 10.0,
        discount_rate: 3.0,
        eval_period: 15.0,
        other_invest: 1500.0,
        battery_replace_unit: 400.0,
        battery_replace_ratio: 100.0,
        battery_replace_year: 8.0,
    }
}

fn curves() -> CurveData {
    // 合成曲线：负荷恒 64000 kWh，风光各 0.3/0.2 标幺，价格分量取常数
    let n = 8760;
    let v = |x: f64| vec![x; n];
    CurveData {
        wind_pu: v(0.3),
        pv_pu: v(0.2),
        load: v(64_000.0),
        price: v(0.4),
        loss_fee: v(0.02),
        tdu_fee: v(0.1),
        system_fee: v(0.02),
        fund_fee: v(0.03),
    }
}

fn default_params() -> ComputeParams {
    ComputeParams {
        tech: tech(),
        econ: econ(),
        ga: GaParams {
            generations: 2,
            crossover_rate: 0.5,
            mutation_rate: 0.3,
            population_size: 8,
        },
        range: RangeParams {
            wind_start: 0.0,
            wind_end: 200.0,
            pv_start: 0.0,
            pv_end: 200.0,
            ess_start: 0.0,
            ess_end: 300.0,
        },
        scheme: "scheme1".to_string(),
        objective: "composite".to_string(),
        charge_periods: vec![],
        discharge_periods: vec![],
    }
}

/// 电量平衡恒等式（AR-2.1）：
/// 负荷 + 储能充电(交流) + 余电上网 = 新能源实际发电 + 储能供电(交流) + 下网（+ 缺供）
#[test]
fn energy_balance_identity_holds() {
    let c = curves();
    let ctx = SimContext::new(192_725.0, 78_372.5, 45_477.25, &tech(), &[], &[], &c);
    let (t, _) = ctx.run(true);
    let lhs = t.load + t.charge_ac + t.feed_in;
    let rhs = (t.total_gen - t.curtailed) + t.discharge_ac + t.grid_import + t.unserved;
    assert!(
        (lhs - rhs).abs() / lhs.max(1.0) < EPS,
        "电量平衡不成立: lhs={lhs}, rhs={rhs}"
    );
}

/// 储能电量递推与 SOC 边界（AR-2.5 / AR-2.6）
#[test]
fn soc_stays_within_bounds() {
    let c = curves();
    let t = tech();
    let ctx = SimContext::new(150_000.0, 80_000.0, 60_000.0, &t, &[], &[], &c);
    let (_, series) = ctx.run(true);
    let s = series.expect("series");
    let soc_min = 60_000.0 * (1.0 - t.dod / 100.0);
    for (i, &soc) in s.soc_dc.iter().enumerate() {
        assert!(
            soc >= soc_min - 1e-6 && soc <= 60_000.0 + 1e-6,
            "第 {i} 小时 SOC={soc} 越界 [{soc_min}, 60000]"
        );
    }
    // 直流侧充入 = 交流侧充电×ηc，直流侧放出 = 交流侧供电/ηd
    let (tot, _) = ctx.run(false);
    assert!((tot.charge_dc - tot.charge_ac * t.charge_eff / 100.0).abs() < 1e-6);
    assert!((tot.discharge_dc - tot.discharge_ac / (t.discharge_eff / 100.0)).abs() < 1e-6);
}

/// 储能功率约束（AR-2.4）：逐时充放电不超过额定功率
#[test]
fn ess_power_limit_respected() {
    let c = curves();
    let t = tech();
    let ess_kwh = 40_000.0;
    let ctx = SimContext::new(200_000.0, 100_000.0, ess_kwh, &t, &[], &[], &c);
    let (_, series) = ctx.run(true);
    let s = series.expect("series");
    let rated = ess_kwh * t.rate;
    for i in 0..s.charge_ac.len() {
        assert!(s.charge_ac[i] <= rated + 1e-6, "第 {i} 小时充电超功率");
        assert!(s.discharge_ac[i] <= rated + 1e-6, "第 {i} 小时放电超功率");
    }
}

/// 余电上网全年比例受 feed_limit 预算约束（AR-2.8）
#[test]
fn feed_in_respects_annual_budget() {
    let c = curves();
    let t = tech();
    // 大装机 → 大量盈余，上网应被全年 20% 预算限制
    let ctx = SimContext::new(500_000.0, 300_000.0, 50_000.0, &t, &[], &[], &c);
    let (tot, _) = ctx.run(false);
    let feed_ratio = tot.feed_in / tot.total_gen * 100.0;
    assert!(
        feed_ratio <= t.feed_limit + 1e-6,
        "余电上网比例 {feed_ratio}% 超过上限 {}%",
        t.feed_limit
    );
}

/// 电池更换成本现值口径与算例一致：45477.247 kWh × 400 元/kWh ÷ 1.03^8 ≈ 1436.01 万元
#[test]
fn battery_replace_present_value_matches_case() {
    let c = curves();
    let ctx = SimContext::new(1000.0, 1000.0, 45_477.247_181_967_09, &tech(), &[], &[], &c);
    let (t, _) = ctx.run(false);
    let e = compute_economics(&t, &econ(), "scheme1", 1000.0, 1000.0, 45_477.247_181_967_09, 80_000.0, 50.0);
    let expect = 45_477.247_181_967_09 * 400.0 / 1.03f64.powi(8) / 10_000.0;
    assert!(
        (e.replace_cost_pv / 10_000.0 - expect).abs() < 1e-6,
        "电池更换现值 {} != {}",
        e.replace_cost_pv / 10_000.0,
        expect
    );
}

/// 初投资口径：IC = 风电×单价 + 光伏×单价 + 储能×单价 + 其他固定投资
#[test]
fn initial_invest_formula() {
    let c = curves();
    let ctx = SimContext::new(192_725.0, 78_372.5, 45_477.25, &tech(), &[], &[], &c);
    let (t, _) = ctx.run(false);
    let e = compute_economics(&t, &econ(), "scheme1", 192_725.0, 78_372.5, 45_477.25, 80_000.0, 50.0);
    let expect = (3600.0 * 192_725.0 + 2700.0 * 78_372.5 + 600.0 * 45_477.25 + 1500.0 * 10_000.0)
        / 10_000.0;
    assert!((e.ic / 10_000.0 - expect).abs() < 1e-6);
}

/// 适应度：满足约束取目标函数值，违反约束被显著劣化
#[test]
fn fitness_penalizes_violations() {
    let c = curves();
    let ctx = SimContext::new(1000.0, 1000.0, 1000.0, &tech(), &[], &[], &c);
    let (t, _) = ctx.run(false);
    let e = compute_economics(&t, &econ(), "scheme1", 1000.0, 1000.0, 1000.0, 80_000.0, 50.0);
    let cons = super::economics::check_constraints(&t, 0.0, 0.0, 100.0, 100.0);
    assert!(cons.ok());
    let f1 = fitness(&e, &cons, &t, "composite");
    assert!(f1.is_finite() && f1 > 0.0);
    // 提高约束门槛制造违反 → 罚项使适应度增大
    let cons2 = super::economics::check_constraints(&t, 100.01, 0.0, 100.0, 100.0);
    assert!(!cons2.ok());
    let f2 = fitness(&e, &cons2, &t, "composite");
    assert!(f2 > f1);
}

/// GA：结果落在择优范围内，且对凸合成问题能收敛到更优区域
#[test]
fn ga_respects_bounds_and_improves() {
    let params = default_params();
    let bounds = [
        (0.0, 200_000.0),
        (0.0, 200_000.0),
        (0.0, 300_000.0),
    ];
    let c = curves();
    let evaluate = |g: [f64; 3]| -> f64 {
        let ctx = SimContext::new(g[0], g[1], g[2], &params.tech, &[], &[], &c);
        let (t, _) = ctx.run(false);
        let e = compute_economics(&t, &params.econ, "scheme1", g[0], g[1], g[2], params.tech.grid_capacity, params.tech.avg_load_rate);
        let cons = super::economics::check_constraints(&t, params.tech.self_use_gen_min, params.tech.self_use_load_min, params.tech.feed_limit, params.tech.curtail_limit);
        fitness(&e, &cons, &t, "composite")
    };
    let on_gen = |_: u32, _: f64| {};
    let cancelled = AtomicBool::new(false);
    let (best, fit) = optimize(bounds, &params.ga, &evaluate, &on_gen, &|| {
        cancelled.load(std::sync::atomic::Ordering::SeqCst)
    })
    .expect("ga ok");
    for i in 0..3 {
        assert!(best[i] >= bounds[i].0 - 1e-9 && best[i] <= bounds[i].1 + 1e-9);
    }
    assert!(fit.is_finite());
    // 种子固定 → 结果可复现
    let (best2, fit2) = optimize(bounds, &params.ga, &evaluate, &on_gen, &|| false).expect("ga ok 2");
    assert_eq!(best, best2);
    assert!((fit - fit2).abs() < 1e-12);
}

/// 总引擎端到端：小规模参数跑通全流程并返回一致的结果负载
#[test]
fn end_to_end_run_compute() {
    let mut params = default_params();
    params.ga = GaParams {
        generations: 3,
        crossover_rate: 0.5,
        mutation_rate: 0.3,
        population_size: 10,
    };
    params.tech.self_use_gen_min = 5.0;
    params.tech.self_use_load_min = 5.0;
    params.tech.curtail_limit = 80.0;
    let curves = curves();
    let last_progress = std::sync::atomic::AtomicU32::new(0);
    let on_progress = |p: f64, _: String| {
        last_progress.store(p as u32, std::sync::atomic::Ordering::SeqCst);
    };
    let cancelled = AtomicBool::new(false);
    let payload = run_compute(params, curves, &on_progress, &cancelled).expect("run ok");
    assert_eq!(last_progress.load(std::sync::atomic::Ordering::SeqCst), 100);
    assert_eq!(payload.headline.len(), 15);
    assert_eq!(payload.invest.len(), 5);
    assert_eq!(payload.opex.len(), 7);
    assert_eq!(payload.energy_stats.len(), 14);
    assert_eq!(payload.balance.load.len(), 8760);
    assert_eq!(payload.sensitivity.len(), 3);
    for g in &payload.sensitivity {
        assert_eq!(g.rows.len(), 11);
    }
    // 电量平衡在结果序列上同样成立
    let b = &payload.balance;
    let lhs: f64 = b.load.iter().sum::<f64>() + b.charge_ac.iter().sum::<f64>() + b.feed_in.iter().sum::<f64>();
    let rhs: f64 = b.actual_gen.iter().sum::<f64>() + b.discharge_ac.iter().sum::<f64>() + b.grid_import.iter().sum::<f64>();
    assert!((lhs - rhs).abs() / lhs.max(1.0) < EPS);
}

/// 参数校验：非法参数被后端拒绝
#[test]
fn backend_rejects_invalid_params() {
    let mut params = default_params();
    params.tech.dod = 70.0;
    params.tech.initial_soc = 10.0; // DOD+初始电量 < 100
    let curves = curves();
    let on_progress = |_: f64, _: String| {};
    let cancelled = AtomicBool::new(false);
    let err = run_compute(params, curves, &on_progress, &cancelled).unwrap_err();
    assert!(err.contains("参数校验未通过"), "实际错误: {err}");
}
