//! 计算总引擎：参数校验 → 遗传算法寻优 → 最优方案 8760h 仿真 →
//! 指标汇总 / 敏感性分析 / 逐时序列构建，产出前端展示与报告导出所需的全部数据。

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::compute::bo;
use crate::compute::economics;
use crate::compute::economics::{
    check_constraints, compute_economics, fitness_search, fitness_visible, ConstraintStatus,
    EconResult,
};
use crate::compute::ga;
use crate::compute::params::{validate_params, ComputeParams, CurveData};
use crate::compute::simulate::{Series, SimContext, Totals};

/// 指标项（前端负责按 decimals 与千分位格式化）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricOut {
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub decimals: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BestOut {
    pub wind_kw: f64,
    pub pv_kw: f64,
    pub ess_kwh: f64,
    pub ess_kw: f64,
    pub fitness: f64,
}

/// 逐时电量平衡序列（kWh，列口径与标准模板 output Sheet 一致）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSeriesOut {
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
    pub end_soc: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensRowOut {
    pub ratio: String,
    pub scale: f64,
    pub fitness: f64,
    pub ok: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensGroupOut {
    pub group: String,
    pub element: String,
    pub unit: String,
    pub color: String,
    pub chart_title: String,
    pub rows: Vec<SensRowOut>,
}

/// 计算结果负载（前端展示 + 报告导出的唯一数据源）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeResultPayload {
    pub best: BestOut,
    pub headline: Vec<MetricOut>,
    pub invest: Vec<MetricOut>,
    pub opex: Vec<MetricOut>,
    pub energy_stats: Vec<MetricOut>,
    pub balance: BalanceSeriesOut,
    pub sensitivity: Vec<SensGroupOut>,
}

fn metric(label: &str, value: f64, unit: &str, decimals: i32) -> MetricOut {
    MetricOut {
        label: label.to_string(),
        value,
        unit: unit.to_string(),
        decimals,
    }
}

/// 单方案完整评估：仿真 → 经济性 → 约束 → 适应度
pub(crate) fn evaluate_config(
    wind_kw: f64,
    pv_kw: f64,
    ess_kwh: f64,
    params: &ComputeParams,
    curves: &CurveData,
) -> (Totals, EconResult, ConstraintStatus, f64) {
    let ctx = SimContext::new(
        wind_kw,
        pv_kw,
        ess_kwh,
        &params.tech,
        &params.charge_periods,
        &params.discharge_periods,
        curves,
    );
    let (totals, _) = ctx.run(false);
    let econ = compute_economics(
        &totals,
        &params.econ,
        &params.scheme,
        wind_kw,
        pv_kw,
        ess_kwh,
        params.tech.grid_capacity,
        params.tech.avg_load_rate,
    );
    let cons = check_constraints(
        &totals,
        params.tech.self_use_gen_min,
        params.tech.self_use_load_min,
        params.tech.feed_limit,
        params.tech.curtail_limit,
    );
    let fit = fitness_search(&econ, &cons, &params.objective);
    (totals, econ, cons, fit)
}

/// 单方案「展示口径」适应度：与 V3.0 敏感性分析表一致（不可行解恒为 0.98）
fn visible_fitness(
    wind_kw: f64,
    pv_kw: f64,
    ess_kwh: f64,
    params: &ComputeParams,
    curves: &CurveData,
) -> (f64, ConstraintStatus) {
    let (_, econ, cons, _) = evaluate_config(wind_kw, pv_kw, ess_kwh, params, curves);
    (
        fitness_visible(&econ, &cons, &params.objective),
        cons,
    )
}

/// 执行完整优化计算
pub fn run_compute(
    params: ComputeParams,
    curves: CurveData,
    on_progress: &(dyn Fn(f64, String) + Send + Sync),
    cancel: &AtomicBool,
) -> Result<ComputeResultPayload, String> {
    let t_start = std::time::Instant::now();
    log::info!("================ 计算开始 ================");
    log::info!(
        "输入曲线：负荷/风电/光伏/电价各 {} 点（合计 {:.2} kWh）",
        curves.load.len(),
        curves.load.iter().sum::<f64>()
    );
    log::info!("完整计算参数：{params:#?}");

    curves.validate()?;
    log::info!("曲线校验通过（长度一致、无非法数值）");
    validate_params(&params)?;
    log::info!("参数校验通过（后端权威校验 FR-5）");

    let is_cancelled = || cancel.load(Ordering::SeqCst);
    on_progress(2.0, "正在初始化仿真引擎…".to_string());

    // 择优范围（MW → kW，MWh → kWh）
    let bounds: [(f64, f64); 3] = [
        (params.range.wind_start * 1000.0, params.range.wind_end * 1000.0),
        (params.range.pv_start * 1000.0, params.range.pv_end * 1000.0),
        (params.range.ess_start * 1000.0, params.range.ess_end * 1000.0),
    ];
    log::info!(
        "择优范围：风电 {:.0}~{:.0} kW / 光伏 {:.0}~{:.0} kW / 储能 {:.0}~{:.0} kWh",
        bounds[0].0, bounds[0].1, bounds[1].0, bounds[1].1, bounds[2].0, bounds[2].1
    );

    // 寻优（每个评估点 = 一组 8760h 仿真 + 经济性评估）
    let objective_label = match params.objective.as_str() {
        "green" => "绿电电价最低",
        "capex" => "初投资最低",
        _ => "综合电价最低",
    };
    let use_bo = params.algorithm != "ga";
    let evaluate = |g: ga::Genes| -> f64 {
        let (_, _, _, fit) = evaluate_config(g[0], g[1], g[2], &params, &curves);
        fit
    };
    // 贝叶斯优化使用「目标值 + 约束违反量」双通道评估（约束贝叶斯优化）
    let evaluate_bo = |g: ga::Genes| -> bo::EvalOut {
        let (_, econ, cons, _) = evaluate_config(g[0], g[1], g[2], &params, &curves);
        (
            economics::objective_value(&econ, &params.objective),
            cons.violation_amount(),
        )
    };

    let (best_genes, best_fitness) = if use_bo {
        // ---- 贝叶斯优化（V3.0 口径）----
        log::info!(
            "贝叶斯优化启动：总评估 {} 次（初始随机采样 {} 点，目标：{objective_label}）",
            params.bo.n_iter,
            params.bo.n_init
        );
        let on_iteration = |evaluated: u32, best_fit: f64| {
            let total = params.bo.n_iter.max(1) as f64;
            let progress = 5.0 + (evaluated as f64 / total) * 85.0;
            log::info!(
                "BO 第 {}/{} 次评估：当前最优目标值 {:.6}（{objective_label}）",
                evaluated, params.bo.n_iter, best_fit
            );
            on_progress(
                progress,
                format!(
                    "贝叶斯优化中：已完成 {}/{} 次评估，当前最优目标值 {:.6}（{}）",
                    evaluated, params.bo.n_iter, best_fit, objective_label
                ),
            );
        };
        let r = bo::optimize(bounds, &params.bo, &evaluate_bo, &on_iteration, &is_cancelled)?;
        log::info!(
            "贝叶斯优化完成（耗时 {:.2}s）：最优点 风电 {:.1} kW / 光伏 {:.1} kW / 储能 {:.1} kWh，适应度 {:.6}",
            t_start.elapsed().as_secs_f64(),
            r.0[0],
            r.0[1],
            r.0[2],
            r.1
        );
        r
    } else {
        // ---- 遗传算法（V2.2 说明书口径）----
        log::info!(
            "遗传算法启动：代数 {} × 种群 {}（目标：{objective_label}）",
            params.ga.generations,
            params.ga.population_size
        );
        let on_generation = |gen: u32, best_fit: f64| {
            let progress = 5.0 + (gen as f64 / params.ga.generations.max(1) as f64) * 85.0;
            log::info!(
                "GA 第 {}/{} 代：当前最优目标值 {:.6}（{objective_label}）",
                gen, params.ga.generations, best_fit
            );
            on_progress(
                progress,
                format!(
                    "遗传算法进化中：第 {}/{} 代，当前最优目标值 {:.6}（{}）",
                    gen, params.ga.generations, best_fit, objective_label
                ),
            );
        };
        let r = ga::optimize(bounds, &params.ga, &evaluate, &on_generation, &is_cancelled)?;
        log::info!(
            "遗传算法完成（耗时 {:.2}s）：最优个体 风电 {:.1} kW / 光伏 {:.1} kW / 储能 {:.1} kWh，适应度 {:.6}",
            t_start.elapsed().as_secs_f64(),
            r.0[0],
            r.0[1],
            r.0[2],
            r.1
        );
        r
    };

    on_progress(92.0, "已找到最优配置，正在生成 8760h 逐时结果…".to_string());

    // 最优方案完整仿真（含逐时序列）
    let (wind_kw, pv_kw, ess_kwh) = (best_genes[0], best_genes[1], best_genes[2]);
    log::info!(
        "最优方案 8760h 仿真开始：风电 {:.1} kW / 光伏 {:.1} kW / 储能 {:.1} kW·{:.1} kWh",
        wind_kw,
        pv_kw,
        ess_kwh * params.tech.rate,
        ess_kwh
    );
    let ctx = SimContext::new(
        wind_kw,
        pv_kw,
        ess_kwh,
        &params.tech,
        &params.charge_periods,
        &params.discharge_periods,
        &curves,
    );
    let (totals, series) = ctx.run(true);
    let series = series.unwrap_or_else(Series::default);
    log::info!(
        "仿真完成：总发电 {:.0} kWh / 弃电 {:.0} kWh / 下网 {:.0} kWh / 负荷 {:.0} kWh / 年末储能剩余 {:.0} kWh",
        totals.total_gen,
        totals.curtailed,
        totals.grid_import,
        totals.load,
        totals.end_soc
    );
    let econ = compute_economics(
        &totals,
        &params.econ,
        &params.scheme,
        wind_kw,
        pv_kw,
        ess_kwh,
        params.tech.grid_capacity,
        params.tech.avg_load_rate,
    );
    log::info!(
        "经济性计算完成：初投资 {:.2} 万元 / 年运行成本 {:.2} 万元 / 综合电价 {:.4} / 绿电电价 {:.4} / 网电电价 {:.4} 元/kWh",
        econ.ic / 10_000.0,
        econ.annual_cost_display / 10_000.0,
        econ.p_composite,
        econ.p_green,
        econ.p_grid
    );

    on_progress(95.0, "正在生成敏感性分析…".to_string());
    log::info!("敏感性分析开始（三要素 ±25%，步长 5%）");
    on_progress(99.0, "正在汇总计算结果…".to_string());

    let payload = build_payload(
        &params, &curves, wind_kw, pv_kw, ess_kwh, &totals, &series, &econ, best_fitness,
    );
    log::info!("结果汇总完成：指标 {} 项 / 敏感性 {} 组", payload.headline.len(), payload.sensitivity.len());

    on_progress(100.0, "计算完成".to_string());
    log::info!("================ 计算结束：总耗时 {:.2}s ================", t_start.elapsed().as_secs_f64());
    Ok(payload)
}

/// 构建计算结果负载（前端展示 + 报告导出的唯一数据源）。
///
/// 供 `run_compute`（遗传算法寻优后）与黄金基准对拍测试（固定配置评估）复用，
/// 保证两条路径产出的指标口径完全一致。
pub fn build_payload(
    params: &ComputeParams,
    curves: &CurveData,
    wind_kw: f64,
    pv_kw: f64,
    ess_kwh: f64,
    totals: &Totals,
    series: &Series,
    econ: &EconResult,
    best_fitness: f64,
) -> ComputeResultPayload {
    let cons = check_constraints(
        totals,
        params.tech.self_use_gen_min,
        params.tech.self_use_load_min,
        params.tech.feed_limit,
        params.tech.curtail_limit,
    );

    let ess_kw = ess_kwh * params.tech.rate;
    let load = totals.load.max(1e-9);
    let grid_ratio = totals.grid_import / load * 100.0;

    let headline = vec![
        metric("最优风电规模", wind_kw / 1000.0, "MW", 2),
        metric("最优光伏规模", pv_kw / 1000.0, "MW", 2),
        metric("最优储能功率", ess_kw / 1000.0, "MW", 2),
        metric("最优储能容量", ess_kwh / 1000.0, "MWh", 2),
        metric("初投资", econ.ic / 10_000.0, "万元", 2),
        metric("年运行成本", econ.annual_cost_display / 10_000.0, "万元", 2),
        metric("下网电量占总用电量比例", grid_ratio, "%", 2),
        metric("自发自用占总用电量比例", cons.self_use_load_ratio, "%", 2),
        metric("弃电率", cons.curtail_ratio, "%", 2),
        metric("余电上网比例", cons.feed_ratio, "%", 2),
        metric("自发自用占总可用电量比例", cons.self_use_gen_ratio, "%", 2),
        metric("周期内总成本", econ.tc / 10_000.0, "万元", 2),
        metric("综合电价", econ.p_composite, "元/kWh", 4),
        metric("绿电电价", econ.p_green, "元/kWh", 4),
        metric("网电电价", econ.p_grid, "元/kWh", 4),
    ];

    let invest = vec![
        metric("风电系统投资", params.econ.wind_invest * wind_kw / 10_000.0, "万元", 2),
        metric("光伏系统投资", params.econ.pv_invest * pv_kw / 10_000.0, "万元", 2),
        metric("储能系统投资", params.econ.ess_invest * ess_kwh / 10_000.0, "万元", 2),
        metric("其他固定投资", params.econ.other_invest, "万元", 2),
        metric("初投资", econ.ic / 10_000.0, "万元", 2),
    ];

    let opex = vec![
        metric("年电网购电成本", econ.grid_buy_cost / 10_000.0, "万元", 2),
        metric("年自发自用输配成本", econ.self_use_cost / 10_000.0, "万元", 2),
        metric("运维成本", econ.maint / 10_000.0, "万元", 2),
        metric("人员工资", econ.salary_cost / 10_000.0, "万元", 2),
        metric("年余电上网收益", econ.feed_revenue / 10_000.0, "万元", 2),
        metric("储能电池更换成本", econ.replace_cost_pv / 10_000.0, "万元", 2),
        metric("年运行成本", econ.annual_cost_display / 10_000.0, "万元", 2),
    ];

    let w = |v: f64| v / 1000.0; // kWh → MWh
    let energy_stats = vec![
        metric("全年风电最大发电量", w(sum(&series.wind)), "MWh", 2),
        metric("全年光伏最大发电量", w(sum(&series.pv)), "MWh", 2),
        metric("全年新能源最大发电量", w(totals.total_gen), "MWh", 2),
        metric("全年弃风弃光电量", w(totals.curtailed), "MWh", 2),
        metric("全年新能源实际发电量", w(totals.total_gen - totals.curtailed), "MWh", 2),
        metric("全年储能充电量（交流侧）", w(totals.charge_ac), "MWh", 2),
        metric("全年储能实际充电量（直流侧）", w(totals.charge_dc), "MWh", 2),
        metric("全年储能实际放电量（直流侧）", w(totals.discharge_dc), "MWh", 2),
        metric("全年储能供电量（交流侧）", w(totals.discharge_ac), "MWh", 2),
        metric("全年下网电量", w(totals.grid_import), "MWh", 2),
        metric("全年负荷用电量", w(totals.load), "MWh", 2),
        metric("下网电量占比", grid_ratio, "%", 2),
        metric("绿电比", 100.0 - grid_ratio, "%", 2),
        metric("储能年末剩余电量", w(totals.end_soc), "MWh", 2),
    ];

    let bounds: [(f64, f64); 3] = [
        (params.range.wind_start * 1000.0, params.range.wind_end * 1000.0),
        (params.range.pv_start * 1000.0, params.range.pv_end * 1000.0),
        (params.range.ess_start * 1000.0, params.range.ess_end * 1000.0),
    ];
    let sensitivity = build_sensitivity(params, curves, wind_kw, pv_kw, ess_kwh, &bounds);

    let balance = BalanceSeriesOut {
        wind: series.wind.clone(),
        pv: series.pv.clone(),
        theory_gen: series.theory_gen.clone(),
        load: series.load.clone(),
        actual_gen: series.actual_gen.clone(),
        charge_ac: series.charge_ac.clone(),
        charge_dc: series.charge_dc.clone(),
        curtailed: series.curtailed.clone(),
        discharge_dc: series.discharge_dc.clone(),
        discharge_ac: series.discharge_ac.clone(),
        grid_import: series.grid_import.clone(),
        feed_in: series.feed_in.clone(),
        soc_dc: series.soc_dc.clone(),
        end_soc: totals.end_soc,
    };

    // 展示口径最优适应度：与敏感性分析表 0% 行严格一致（不可行时为 0.98）
    let shown_fitness = visible_fitness(wind_kw, pv_kw, ess_kwh, params, curves).0;
    if (shown_fitness - best_fitness).abs() > 1e-9 {
        log::info!(
            "最优解适应度口径差异：寻优口径 {:.6} → 展示口径 {:.6}（不可行解展示为固定 0.98）",
            best_fitness,
            shown_fitness
        );
    }

    ComputeResultPayload {
        best: BestOut {
            wind_kw,
            pv_kw,
            ess_kwh,
            ess_kw,
            fitness: if shown_fitness.is_finite() { shown_fitness } else { f64::MAX },
        },
        headline,
        invest,
        opex,
        energy_stats,
        balance,
        sensitivity,
    }
}

fn sum(v: &[f64]) -> f64 {
    v.iter().sum()
}

/// 敏感性分析（AR-1 / 结果展示）：固定两要素 · 变动单一要素，±25%、步长 5%
fn build_sensitivity(
    params: &ComputeParams,
    curves: &CurveData,
    wind_kw: f64,
    pv_kw: f64,
    ess_kwh: f64,
    bounds: &[(f64, f64); 3],
) -> Vec<SensGroupOut> {
    let ratios = [-25.0f64, -20.0, -15.0, -10.0, -5.0, 0.0, 5.0, 10.0, 15.0, 20.0, 25.0];

    // 展示口径：不可行解适应度恒为 0.98（与 V3.0 敏感性分析表一致）
    let eval = |wind: f64, pv: f64, ess: f64| -> (f64, ConstraintStatus) {
        visible_fitness(wind, pv, ess, params, curves)
    };

    let build_group = |group: &str,
                       element: &str,
                       unit: &str,
                       chart_title: &str,
                       row_fit: &dyn Fn(f64) -> (f64, f64, ConstraintStatus)| -> SensGroupOut {
        let rows = ratios
            .iter()
            .map(|&pct| {
                let (scale, fit, cons) = row_fit(pct);
                SensRowOut {
                    ratio: format!("{:.1}%", pct),
                    scale,
                    fitness: if fit.is_finite() { fit } else { f64::MAX },
                    ok: cons.ok(),
                    note: if cons.ok() {
                        "满足所有约束".to_string()
                    } else {
                        cons.violations.join("、")
                    },
                }
            })
            .collect();
        SensGroupOut {
            group: group.to_string(),
            element: element.to_string(),
            unit: unit.to_string(),
            color: "#3b82f6".to_string(),
            chart_title: chart_title.to_string(),
            rows,
        }
    };

    vec![
        // 固定光储 · 变动风电
        build_group(
            "固定光储 · 变动风电",
            "风电规模",
            "kW",
            "风电容量变动敏感性曲线",
            &|pct| {
                let w = (wind_kw * (1.0 + pct / 100.0)).clamp(bounds[0].0, bounds[0].1);
                let (fit, cons) = eval(w, pv_kw, ess_kwh);
                (w, fit, cons)
            },
        ),
        // 固定风储 · 变动光伏
        build_group(
            "固定风储 · 变动光伏",
            "光伏规模",
            "kW",
            "光伏容量变动敏感性曲线",
            &|pct| {
                let p = (pv_kw * (1.0 + pct / 100.0)).clamp(bounds[1].0, bounds[1].1);
                let (fit, cons) = eval(wind_kw, p, ess_kwh);
                (p, fit, cons)
            },
        ),
        // 固定风光 · 变动储能
        build_group(
            "固定风光 · 变动储能",
            "储能容量",
            "kWh",
            "储能容量变动敏感性曲线",
            &|pct| {
                let e = (ess_kwh * (1.0 + pct / 100.0)).clamp(bounds[2].0, bounds[2].1);
                let (fit, cons) = eval(wind_kw, pv_kw, e);
                (e, fit, cons)
            },
        ),
    ]
}
