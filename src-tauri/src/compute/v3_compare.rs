//! 与「绿电直连新能源优化配置 V3.0」真实输出的输出对比测试
//!
//! 基准来源：`http://150.158.94.206` V3.0 实跑结果
//! （输入 = curvetemplate_ldzl_3.0.xlsx 曲线 + inputtemplate_ldzl_3.0.xlsx 默认参数，
//!   择优范围 1~500，贝叶斯优化 总评估 100 / 初始采样 20，方案一，综合电价最低）。
//!
//! 夹具由 `tools/extract_v3_baseline.py` 从真实输出 xlsx 提取：
//! - `v3_fixture.json`  参数 + 8760h 输入曲线
//! - `v3_expected.json` 输出指标（34 项，公式无缓存值者由逐时表独立复算）
//! - `v3_hourly.json`   逐时电量平衡表（8760 行 × 13 列）
//!
//! 对比分两层：
//! A. **同配置对拍**（确定性）：用 V3.0 报告的最优配置驱动本地引擎，
//!    逐时 8760h × 13 列与全部输出指标逐项比对 → 验证仿真/经济/约束逻辑一致。
//! B. **寻优质量对比**（统计性）：本地 BO(100/20) 在同等条件下的最优综合电价
//!    对比 V3.0 报告的 0.2353358929121689 → 验证寻优不劣于真实软件。
//!
//! A 层要求浮点级一致，B 层只要求不劣（两者均为启发式，最优点不必相同）。

use std::sync::atomic::AtomicBool;

use super::economics::compute_economics;
use super::engine::{build_payload, evaluate_config, run_compute};
use super::params::{ComputeParams, CurveData};
use super::simulate::SimContext;

const FIXTURE: &str = include_str!("v3_fixture.json");
const EXPECTED: &str = include_str!("v3_expected.json");
const HOURLY: &str = include_str!("v3_hourly.json");

/// 逐时数值允许的相对偏差
const HOURLY_REL_TOL: f64 = 1e-6;
/// 逐时数值允许的绝对偏差（真实文件中存在 1e-13 量级浮点残差，
/// 例如「下网电量」列出现过 2.27e-13；与 golden.rs 口径一致）
const HOURLY_ABS_TOL: f64 = 1e-6;
/// 指标允许的相对偏差
const METRIC_REL_TOL: f64 = 1e-9;

struct Fixture {
    params: ComputeParams,
    curves: CurveData,
}

fn fixture() -> Fixture {
    let v: serde_json::Value = serde_json::from_str(FIXTURE).expect("v3 fixture json");
    let params: ComputeParams = serde_json::from_value(v["params"].clone()).expect("params");
    let curves: CurveData = serde_json::from_value(v["curves"].clone()).expect("curves");
    Fixture { params, curves }
}

fn expected() -> std::collections::BTreeMap<String, Option<f64>> {
    serde_json::from_str(EXPECTED).expect("v3 expected json")
}

struct GoldenHourly {
    columns: Vec<String>,
    rows: Vec<Vec<f64>>,
}

fn hourly() -> GoldenHourly {
    let v: serde_json::Value = serde_json::from_str(HOURLY).expect("v3 hourly json");
    let columns: Vec<String> = v["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .map(|c| c.as_str().expect("col").to_string())
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
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), 8760, "逐时基准应为 8760 行");
    GoldenHourly { columns, rows }
}

/// 真实软件报告的最优配置（MW / MWh → kW / kWh）
fn real_best() -> (f64, f64, f64) {
    let e = expected();
    let get = |k: &str| -> f64 {
        e.get(k)
            .and_then(|v| *v)
            .unwrap_or_else(|| panic!("基准缺少指标「{k}」"))
    };
    (
        get("最优风电规模") * 1000.0,
        get("最优光伏规模") * 1000.0,
        get("最优储能容量") * 1000.0,
    )
}

fn rel_err(a: f64, b: f64) -> f64 {
    let scale = a.abs().max(b.abs()).max(1e-12);
    (a - b).abs() / scale
}

/// 逐时数值一致性判定：绝对偏差或相对偏差任一满足即视为一致
/// （避免 1e-13 级浮点残差被相对偏差放大为"超差"）
fn hourly_close(a: f64, b: f64) -> bool {
    (a - b).abs() <= HOURLY_ABS_TOL || rel_err(a, b) <= HOURLY_REL_TOL
}

/// A-1：以 V3.0 报告的最优配置为输入，逐时 8760h × 13 列全量对拍
#[test]
fn v3_hourly_series_matches_real_software() {
    let f = fixture();
    let g = hourly();
    let (w, p, e) = real_best();

    let ctx = SimContext::new(
        w,
        p,
        e,
        &f.params.tech,
        &f.params.charge_periods,
        &f.params.discharge_periods,
        &f.curves,
    );
    let (_, series) = ctx.run(true);
    let s = series.expect("series");

    // 本地 Series 字段 → 基准列索引
    //
    // 分两组：
    // - strict：与「余电上网分配」无关，逐小时应精确一致
    // - alloc ：受余电上网分配影响的 3 列。全年上网电量按 20% 预算约束分配，
    //   同电价小时之间存在 tie-break，不同实现的分配顺序可能不同，
    //   但总量守恒（与 golden.rs 既有口径一致），故按全年总量对拍。
    let strict: Vec<(&str, &Vec<f64>)> = vec![
        ("风电发电量（kWh）", &s.wind),
        ("光伏发电量（kWh）", &s.pv),
        ("新能源理论发电量（kWh）", &s.theory_gen),
        ("用户负荷（kWh）", &s.load),
        ("该小时段储能充电量（交流侧）（kWh）", &s.charge_ac),
        ("该小时段储能实际充电量（直流侧）（kWh）", &s.charge_dc),
        ("该小时段储能放电量（直流侧）（kWh）", &s.discharge_dc),
        ("该小时段储能对外供电量（交流测）（kWh）", &s.discharge_ac),
        ("该小时段下网电量(kWh)", &s.grid_import),
        ("储能可用电量（直流侧）（kWh）", &s.soc_dc),
    ];
    let alloc: Vec<(&str, &Vec<f64>)> = vec![
        ("该小时段新能源实际发电量（kWh）", &s.actual_gen),
        ("该小时段弃风弃光电量(kWh)", &s.curtailed),
        ("该小时段余电上网电量(kWh)", &s.feed_in),
    ];

    let col_of = |name: &str| -> usize {
        g.columns
            .iter()
            .position(|c| c == name)
            .unwrap_or_else(|| panic!("基准缺少列「{name}」"))
    };

    let mut report = String::new();
    // ---- 严格组：8760h 逐时精确对拍 ----
    for (name, mine) in &strict {
        let ci = col_of(name);
        let mut worst = 0.0f64;
        let mut bad = 0usize;
        for i in 0..8760 {
            let err = (mine[i] - g.rows[i][ci]).abs();
            if err > worst {
                worst = err;
            }
            if !hourly_close(mine[i], g.rows[i][ci]) {
                bad += 1;
            }
        }
        report.push_str(&format!(
            "  [逐时] {:<38} 最大绝对偏差 {:.3e}  超差 {} 点\n",
            name, worst, bad
        ));
        assert!(
            bad == 0,
            "逐时列「{name}」与 V3.0 输出存在 {bad} 个超差点（最大绝对偏差 {worst:.3e}）"
        );
    }

    // ---- 分配组：全年总量守恒对拍，并统计逐时差异小时数 ----
    for (name, mine) in &alloc {
        let ci = col_of(name);
        let mine_sum: f64 = mine.iter().sum();
        let base_sum: f64 = g.rows.iter().map(|r| r[ci]).sum();
        let diff_hours = (0..8760)
            .filter(|&i| !hourly_close(mine[i], g.rows[i][ci]))
            .count();
        report.push_str(&format!(
            "  [总量] {:<38} 本地 {:>18.3}  基准 {:>18.3}  相对偏差 {:.3e}  逐时差异 {} 小时\n",
            name,
            mine_sum,
            base_sum,
            rel_err(mine_sum, base_sum),
            diff_hours
        ));
        assert!(
            rel_err(mine_sum, base_sum) < METRIC_REL_TOL,
            "列「{name}」全年总量与 V3.0 不一致：本地 {mine_sum} vs 基准 {base_sum}"
        );
    }

    println!("[V3 对拍] 同配置逐时 8760h 对拍通过：\n{report}");
}

/// A-2：以 V3.0 报告的最优配置为输入，输出指标逐项对拍
#[test]
fn v3_metrics_match_real_software() {
    let f = fixture();
    let e = expected();
    let (w, p, ess) = real_best();

    let ctx = SimContext::new(
        w,
        p,
        ess,
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
        w,
        p,
        ess,
        f.params.tech.grid_capacity,
        f.params.tech.avg_load_rate,
    );
    let cons = super::economics::check_constraints(
        &totals,
        f.params.tech.self_use_gen_min,
        f.params.tech.self_use_load_min,
        f.params.tech.feed_limit,
        f.params.tech.curtail_limit,
    );
    assert!(cons.ok(), "V3.0 最优配置在本地引擎下不应违反约束：{:?}", cons.violations);

    let payload = build_payload(
        &f.params,
        &f.curves,
        w,
        p,
        ess,
        &totals,
        &series,
        &econ,
        f64::INFINITY,
    );

    // 本地指标 → 基准指标名
    let pairs: Vec<(&str, f64)> = {
        let mut v: Vec<(&str, f64)> = Vec::new();
        for m in &payload.energy_stats {
            v.push((m.label.as_str(), m.value));
        }
        for m in &payload.headline {
            v.push((m.label.as_str(), m.value));
        }
        for m in &payload.invest {
            v.push((m.label.as_str(), m.value));
        }
        for m in &payload.opex {
            v.push((m.label.as_str(), m.value));
        }
        v
    };

    // 基准名 → 本地名（口径相同但命名不同）
    let aliases: &[(&str, &str)] = &[
        ("评价周期内总成本", "周期内总成本"),
        ("评价周期内平均综合电价", "综合电价"),
        ("评价周期内平均绿电电价", "绿电电价"),
        ("评价周期内平均网电电价", "网电电价"),
        ("下网电量占比", "下网电量占总用电量比例"),
        ("全年新能源实际发电量", "全年新能源实际发电量"),
        ("储能年末剩余电量", "储能年末剩余电量"),
    ];

    let local_of = |base: &str| -> Option<f64> {
        if let Some((_, mine)) = pairs.iter().find(|(l, _)| *l == base) {
            return Some(*mine);
        }
        aliases
            .iter()
            .find(|(b, _)| *b == base)
            .and_then(|(_, l)| pairs.iter().find(|(n, _)| n == l))
            .map(|(_, v)| *v)
    };

    let mut compared = 0usize;
    let mut report = String::new();
    for (name, exp) in &e {
        let exp = match exp {
            Some(v) => *v,
            None => continue,
        };
        let mine = match local_of(name) {
            Some(v) => v,
            None => continue,
        };
        compared += 1;
        let err = rel_err(mine, exp);
        report.push_str(&format!(
            "  {:<28} 本地 {:>20.6}  基准 {:>20.6}  相对偏差 {:.3e}\n",
            name, mine, exp, err
        ));
        assert!(
            err < METRIC_REL_TOL,
            "指标「{name}」与 V3.0 输出不一致：本地 {mine} vs 基准 {exp}（相对偏差 {err:.3e}）"
        );
    }
    println!("[V3 对拍] 同配置输出指标对拍通过（{compared} 项）：\n{report}");
    assert!(compared >= 20, "参与对拍的指标过少（{compared} 项）");
}

/// B：本地贝叶斯优化在同等条件下寻优质量不劣于 V3.0 报告值
#[test]
fn v3_optimization_not_worse_than_real_software() {
    let f = fixture();
    let e = expected();
    let real_fit = e
        .get("评价周期内平均综合电价")
        .and_then(|v| *v)
        .expect("基准综合电价");

    let on_progress = |_: f64, _: String| {};
    let cancelled = AtomicBool::new(false);
    let payload = run_compute(f.params, f.curves, &on_progress, &cancelled).expect("run_compute ok");

    let mine = payload.best.fitness;
    println!(
        "[V3 对拍] 寻优质量：本地 BO {:.9}（风电 {:.1} kW / 光伏 {:.1} kW / 储能 {:.1} kWh）\
         vs V3.0 {:.9}",
        mine, payload.best.wind_kw, payload.best.pv_kw, payload.best.ess_kwh, real_fit
    );
    assert!(
        mine <= real_fit + 1e-9,
        "本地寻优结果 {mine} 劣于 V3.0 报告值 {real_fit}"
    );
}

/// 敏感性分析口径：不可行方案适应度恒为 0.98（与 V3.0 敏感性表一致）
#[test]
fn v3_sensitivity_uses_0_98_sentinel() {
    let f = fixture();
    let on_progress = |_: f64, _: String| {};
    let cancelled = AtomicBool::new(false);
    let payload = run_compute(f.params, f.curves, &on_progress, &cancelled).expect("run_compute ok");

    let mut infeasible = 0usize;
    for g in &payload.sensitivity {
        assert_eq!(g.rows.len(), 11, "敏感性表每组应 11 行（±25% 步长 5%）");
        for r in &g.rows {
            if !r.ok {
                infeasible += 1;
                assert!(
                    (r.fitness - 0.98).abs() < 1e-12,
                    "不可行方案适应度应为 0.98，实际 {}（{}）",
                    r.fitness,
                    r.note
                );
            }
        }
    }
    println!(
        "[V3 对拍] 敏感性分析：3 组 × 11 行，其中不可行 {} 行，适应度均为 0.98",
        infeasible
    );
}

/// 目标函数在两个口径下的关系：展示口径 ≤ 寻优口径（可行解相同，不可行解被抬升）
#[test]
fn v3_visible_fitness_never_exceeds_search_fitness() {
    let f = fixture();
    let (w, p, ess) = real_best();
    let (_, econ, cons, search_fit) = evaluate_config(w, p, ess, &f.params, &f.curves);
    let visible = super::economics::fitness_visible(&econ, &cons, &f.params.objective);
    assert!(cons.ok());
    assert!(
        (visible - search_fit).abs() < 1e-12,
        "可行解两个口径应一致：visible={visible} search={search_fit}"
    );
}
