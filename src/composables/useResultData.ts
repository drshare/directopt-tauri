/**
 * 计算结果共享数据
 * 由 ResultSection（展示）与 resultExport（导出）共用，保证界面与报告口径一致。
 * 敏感性分析为演示数据（来源：原型图算例）；逐时电量平衡来源：
 * docs/20260829185838_curvetemplate_ldzl_3.0_16666555869.xlsx → output Sheet。
 * 接入真实计算接口后由后端结果替换。
 */
import { hourlyBalance } from "@/data/hourlyBalance";
import { params } from "@/composables/useParams";
import { fmtNum, formatMetricItem, type ComputeResultPayload } from "@/lib/computeTypes";

/** 敏感性分析行 */
export interface SensitivityRow {
  /** 变动比例，如 "-25.0%" */
  ratio: string;
  /** 对应要素规模（字符串数值，千分位格式） */
  scale: string;
  /** 适应度（目标函数值） */
  fitness: string;
  /** 备注与约束状态 */
  note: string;
  /** 是否满足所有约束 */
  ok: boolean;
}

/** 敏感性分析分组：固定两要素 · 变动单一要素（±25%，步长 5%） */
export interface SensitivityGroup {
  group: string;
  /** 变动要素名称（表头用），如 "风电规模" */
  element: string;
  unit: string;
  color: string;
  /** 对应敏感性曲线标题 */
  chartTitle: string;
  rows: SensitivityRow[];
}

const CONSTRAINT_OK = "满足所有约束";
const CONSTRAINT_FAIL = "弃电率超标";

/** 敏感性分析：初始为演示数据（原型图算例），计算完成后由后端结果替换 */
export let sensitivityGroups: SensitivityGroup[] = [
  {
    group: "固定光储 · 变动风电",
    element: "风电规模",
    unit: "kW",
    color: "#3b82f6",
    chartTitle: "风电容量变动敏感性曲线",
    rows: [
      { ratio: "-25.0%", scale: "144,543.76", fitness: "0.253462", note: CONSTRAINT_OK, ok: true },
      { ratio: "-20.0%", scale: "154,180.01", fitness: "0.249017", note: CONSTRAINT_OK, ok: true },
      { ratio: "-15.0%", scale: "163,816.26", fitness: "0.244879", note: CONSTRAINT_OK, ok: true },
      { ratio: "-10.0%", scale: "173,452.51", fitness: "0.24106", note: CONSTRAINT_OK, ok: true },
      { ratio: "-5.0%", scale: "183,088.76", fitness: "0.237514", note: CONSTRAINT_OK, ok: true },
      { ratio: "0.0%", scale: "192,725.01", fitness: "0.235336", note: CONSTRAINT_OK, ok: true },
      { ratio: "5.0%", scale: "202,361.26", fitness: "0.234807", note: CONSTRAINT_OK, ok: true },
      { ratio: "10.0%", scale: "211,997.51", fitness: "0.234702", note: CONSTRAINT_OK, ok: true },
      { ratio: "15.0%", scale: "221,633.76", fitness: "0.98", note: CONSTRAINT_FAIL, ok: false },
      { ratio: "20.0%", scale: "231,270.01", fitness: "0.98", note: CONSTRAINT_FAIL, ok: false },
      { ratio: "25.0%", scale: "240,906.26", fitness: "0.98", note: CONSTRAINT_FAIL, ok: false },
    ],
  },
  {
    group: "固定风储 · 变动光伏",
    element: "光伏规模",
    unit: "kW",
    color: "#3b82f6",
    chartTitle: "光伏容量变动敏感性曲线",
    rows: [
      { ratio: "-25.0%", scale: "58,779.37", fitness: "0.237863", note: CONSTRAINT_OK, ok: true },
      { ratio: "-20.0%", scale: "62,698.00", fitness: "0.23726", note: CONSTRAINT_OK, ok: true },
      { ratio: "-15.0%", scale: "66,616.62", fitness: "0.236671", note: CONSTRAINT_OK, ok: true },
      { ratio: "-10.0%", scale: "70,535.25", fitness: "0.236132", note: CONSTRAINT_OK, ok: true },
      { ratio: "-5.0%", scale: "74,453.87", fitness: "0.235657", note: CONSTRAINT_OK, ok: true },
      { ratio: "0.0%", scale: "78,372.50", fitness: "0.235336", note: CONSTRAINT_OK, ok: true },
      { ratio: "5.0%", scale: "82,291.12", fitness: "0.235127", note: CONSTRAINT_OK, ok: true },
      { ratio: "10.0%", scale: "86,209.75", fitness: "0.235061", note: CONSTRAINT_OK, ok: true },
      { ratio: "15.0%", scale: "90,128.37", fitness: "0.235099", note: CONSTRAINT_OK, ok: true },
      { ratio: "20.0%", scale: "94,047.00", fitness: "0.235227", note: CONSTRAINT_OK, ok: true },
      { ratio: "25.0%", scale: "97,965.62", fitness: "0.235402", note: CONSTRAINT_OK, ok: true },
    ],
  },
  {
    group: "固定风光 · 变动储能",
    element: "储能容量",
    unit: "kWh",
    color: "#3b82f6",
    chartTitle: "储能容量变动敏感性曲线",
    rows: [
      { ratio: "-25.0%", scale: "34,107.94", fitness: "0.235305", note: CONSTRAINT_OK, ok: true },
      { ratio: "-20.0%", scale: "36,381.80", fitness: "0.235301", note: CONSTRAINT_OK, ok: true },
      { ratio: "-15.0%", scale: "38,655.66", fitness: "0.2353", note: CONSTRAINT_OK, ok: true },
      { ratio: "-10.0%", scale: "40,929.52", fitness: "0.235303", note: CONSTRAINT_OK, ok: true },
      { ratio: "-5.0%", scale: "43,203.38", fitness: "0.235317", note: CONSTRAINT_OK, ok: true },
      { ratio: "0.0%", scale: "45,477.24", fitness: "0.235336", note: CONSTRAINT_OK, ok: true },
      { ratio: "5.0%", scale: "47,751.10", fitness: "0.235359", note: CONSTRAINT_OK, ok: true },
      { ratio: "10.0%", scale: "50,024.96", fitness: "0.235383", note: CONSTRAINT_OK, ok: true },
      { ratio: "15.0%", scale: "52,298.82", fitness: "0.235411", note: CONSTRAINT_OK, ok: true },
      { ratio: "20.0%", scale: "54,572.68", fitness: "0.235444", note: CONSTRAINT_OK, ok: true },
      { ratio: "25.0%", scale: "56,848.55", fitness: "0.235452", note: CONSTRAINT_OK, ok: true },
    ],
  },
];

/** 敏感性分析变动比例标签 */
export const ratioLabels = [
  "-25%",
  "-20%",
  "-15%",
  "-10%",
  "-5%",
  "0%",
  "5%",
  "10%",
  "15%",
  "20%",
  "25%",
];

export interface ChartSeries {
  name: string;
  color: string;
  values: number[];
}

/** 全年 8760h 电量平衡曲线序列（单位：kWh），计算完成后由后端结果替换 */
export let balanceSeries: ChartSeries[] = [
  { name: "风电发电量", color: "#059669", values: hourlyBalance.wind },
  { name: "光伏发电量", color: "#eab308", values: hourlyBalance.pv },
  { name: "用户负荷", color: "#e11d48", values: hourlyBalance.load },
  { name: "下网电量", color: "#7c3aed", values: hourlyBalance.gridImport },
  { name: "储能充电量（交流侧）", color: "#2563eb", values: hourlyBalance.charge },
  { name: "储能放电量（交流侧）", color: "#0ea5e9", values: hourlyBalance.discharge },
  { name: "余电上网电量", color: "#14b8a6", values: hourlyBalance.feedIn },
  { name: "弃风弃光电量", color: "#f97316", values: hourlyBalance.curtailed },
];

/** 全年逐时小时序号标签（0 ~ 8759） */
export const hourLabels: string[] = Array.from({ length: 8760 }, (_, i) => String(i));

/** 储能可用容量（kWh），与"最优储能容量 45.48 MWh"同源（曲线模板算例，全精度） */
const STORAGE_CAPACITY_KWH = 45477.24718196709;

function numOr(v: unknown, fallback: number): number {
  const n = Number(v);
  return Number.isFinite(n) && String(v).trim() !== "" ? n : fallback;
}

/**
 * 派生逐时序列（单位 kWh），与标准模板 output Sheet 列口径一致：
 * - 新能源理论发电量 = 风电 + 光伏；实际发电量 = 理论 − 弃电
 * - 储能实际充电量（直流侧）= 交流侧充电量 × 充电效率
 * - 储能放电量（直流侧）= 交流侧供电量 ÷ 放电效率
 * - 储能可用电量（直流侧）= 初始电量 + 累计（充 DC − 放 DC）
 */
export let derivedBalance = (() => {
  const chargeEff = numOr(params.chargeEff, 93) / 100;
  const dischargeEff = numOr(params.dischargeEff, 92) / 100;
  const initialSoc = numOr(params.initialSoc, 20) / 100;
  const n = hourlyBalance.load.length;
  const theoryGen: number[] = new Array(n);
  const actualGen: number[] = new Array(n);
  const chargeDC: number[] = new Array(n);
  const dischargeDC: number[] = new Array(n);
  const socDC: number[] = new Array(n);
  let soc = initialSoc * STORAGE_CAPACITY_KWH;
  for (let i = 0; i < n; i++) {
    const theory = hourlyBalance.wind[i] + hourlyBalance.pv[i];
    theoryGen[i] = theory;
    actualGen[i] = theory - hourlyBalance.curtailed[i];
    const cdc = hourlyBalance.charge[i] * chargeEff;
    const ddc = hourlyBalance.discharge[i] / dischargeEff;
    chargeDC[i] = cdc;
    dischargeDC[i] = ddc;
    soc += cdc - ddc;
    socDC[i] = soc;
  }
  return { theoryGen, actualGen, chargeDC, dischargeDC, socDC };
})();

/**
 * 逐时电量平衡全量序列（标准模板 output Sheet 列序，单位 kWh），
 * 在 balanceSeries 基础上补充理论/实际发电量、直流侧电量与储能可用电量，供导出使用。
 */
export let fullBalanceSeries: ChartSeries[] = [
  { name: "风电发电量", color: "#059669", values: hourlyBalance.wind },
  { name: "光伏发电量", color: "#eab308", values: hourlyBalance.pv },
  { name: "新能源理论发电量", color: "#10b981", values: derivedBalance.theoryGen },
  { name: "用户负荷", color: "#e11d48", values: hourlyBalance.load },
  { name: "新能源实际发电量", color: "#22c55e", values: derivedBalance.actualGen },
  { name: "储能充电量（交流侧）", color: "#2563eb", values: hourlyBalance.charge },
  { name: "储能实际充电量（直流侧）", color: "#1d4ed8", values: derivedBalance.chargeDC },
  { name: "弃风弃光电量", color: "#f97316", values: hourlyBalance.curtailed },
  { name: "储能放电量（直流侧）", color: "#0284c7", values: derivedBalance.dischargeDC },
  { name: "储能对外供电量（交流侧）", color: "#0ea5e9", values: hourlyBalance.discharge },
  { name: "下网电量", color: "#7c3aed", values: hourlyBalance.gridImport },
  { name: "余电上网电量", color: "#14b8a6", values: hourlyBalance.feedIn },
  { name: "储能可用电量（直流侧）", color: "#8b5cf6", values: derivedBalance.socDC },
];

/** 全年电量指标（单位：MWh / %），初始为演示数据，计算完成后由后端结果替换 */
export let energyStats: { label: string; value: string }[] = (() => {
  const sum = (a: number[]) => a.reduce((x, y) => x + y, 0);
  const fmt = (v: number) =>
    v.toLocaleString("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const wind = sum(hourlyBalance.wind) / 1000;
  const pv = sum(hourlyBalance.pv) / 1000;
  const theory = wind + pv;
  const curtailed = sum(hourlyBalance.curtailed) / 1000;
  const grid = sum(hourlyBalance.gridImport) / 1000;
  const load = sum(hourlyBalance.load) / 1000;
  const gridRatio = (grid / load) * 100;
  const chargeDC = sum(derivedBalance.chargeDC) / 1000;
  const dischargeDC = sum(derivedBalance.dischargeDC) / 1000;
  return [
    { label: "全年风电最大发电量", value: `${fmt(wind)} MWh` },
    { label: "全年光伏最大发电量", value: `${fmt(pv)} MWh` },
    { label: "全年新能源最大发电量", value: `${fmt(theory)} MWh` },
    { label: "全年弃风弃光电量", value: `${fmt(curtailed)} MWh` },
    { label: "全年新能源实际发电量", value: `${fmt(theory - curtailed)} MWh` },
    { label: "全年储能充电量（交流侧）", value: `${fmt(sum(hourlyBalance.charge) / 1000)} MWh` },
    { label: "全年储能实际充电量（直流侧）", value: `${fmt(chargeDC)} MWh` },
    { label: "全年储能实际放电量（直流侧）", value: `${fmt(dischargeDC)} MWh` },
    { label: "全年储能供电量（交流侧）", value: `${fmt(sum(hourlyBalance.discharge) / 1000)} MWh` },
    { label: "全年下网电量", value: `${fmt(grid)} MWh` },
    { label: "全年负荷用电量", value: `${fmt(load)} MWh` },
    { label: "下网电量占比", value: `${gridRatio.toFixed(2)}%` },
    { label: "绿电比", value: `${(100 - gridRatio).toFixed(2)}%` },
    { label: "储能年末剩余电量", value: `${fmt(hourlyBalance.endSoc / 1000)} MWh` },
  ];
})();

/**
 * 用后端计算结果替换当前数据源（全部计算流程在 Rust 后端执行后调用）：
 * - 派生逐时序列（理论/实际发电量、直流侧电量、储能可用电量）由后端按效率参数逐时推算；
 * - 逐时电量平衡 13 列序列、全年电量指标、敏感性分析均来自后端结果负载；
 * - ResultSection / resultExport 的模块级引用在重新计算时随组件重建/调用时读取而更新。
 */
export function applyComputeResult(payload: ComputeResultPayload): void {
  const b = payload.balance;

  derivedBalance = {
    theoryGen: b.theoryGen,
    actualGen: b.actualGen,
    chargeDC: b.chargeDc,
    dischargeDC: b.dischargeDc,
    socDC: b.socDc,
  };

  balanceSeries = [
    { name: "风电发电量", color: "#059669", values: b.wind },
    { name: "光伏发电量", color: "#eab308", values: b.pv },
    { name: "用户负荷", color: "#e11d48", values: b.load },
    { name: "下网电量", color: "#7c3aed", values: b.gridImport },
    { name: "储能充电量（交流侧）", color: "#2563eb", values: b.chargeAc },
    { name: "储能放电量（交流侧）", color: "#0ea5e9", values: b.dischargeAc },
    { name: "余电上网电量", color: "#14b8a6", values: b.feedIn },
    { name: "弃风弃光电量", color: "#f97316", values: b.curtailed },
  ];

  fullBalanceSeries = [
    { name: "风电发电量", color: "#059669", values: b.wind },
    { name: "光伏发电量", color: "#eab308", values: b.pv },
    { name: "新能源理论发电量", color: "#10b981", values: b.theoryGen },
    { name: "用户负荷", color: "#e11d48", values: b.load },
    { name: "新能源实际发电量", color: "#22c55e", values: b.actualGen },
    { name: "储能充电量（交流侧）", color: "#2563eb", values: b.chargeAc },
    { name: "储能实际充电量（直流侧）", color: "#1d4ed8", values: b.chargeDc },
    { name: "弃风弃光电量", color: "#f97316", values: b.curtailed },
    { name: "储能放电量（直流侧）", color: "#0284c7", values: b.dischargeDc },
    { name: "储能对外供电量（交流侧）", color: "#0ea5e9", values: b.dischargeAc },
    { name: "下网电量", color: "#7c3aed", values: b.gridImport },
    { name: "余电上网电量", color: "#14b8a6", values: b.feedIn },
    { name: "储能可用电量（直流侧）", color: "#8b5cf6", values: b.socDc },
  ];

  energyStats = payload.energyStats.map(formatMetricItem);

  sensitivityGroups = payload.sensitivity.map((g) => ({
    group: g.group,
    element: g.element,
    unit: g.unit,
    color: g.color,
    chartTitle: g.chartTitle,
    rows: g.rows.map((r) => ({
      ratio: r.ratio,
      scale: fmtNum(r.scale, 2),
      fitness: fmtNum(r.fitness, 6),
      note: r.note,
      ok: r.ok,
    })),
  }));
}
