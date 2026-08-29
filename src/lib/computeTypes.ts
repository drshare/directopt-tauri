/**
 * 后端计算通道类型与数据组装/格式化
 * 全部计算流程由 Rust 后端执行（src-tauri/src/compute/），本文件仅负责：
 * 1. 前端参数表单 → 后端 ComputeParams 组装（数值解析）
 * 2. 输入曲线数据组装（当前使用内置算例曲线，文件解析接入后替换）
 * 3. 后端结果负载 → 界面展示格式化（千分位 + 单位）
 */
import { params } from "@/composables/useParams";
import { activeCurve } from "@/composables/useUploadedFiles";
import { curveTemplate } from "@/data/curveTemplate";
import { hourlyBalance } from "@/data/hourlyBalance";

// ---- 后端结果负载类型（与 Rust serde camelCase 输出一一对应）----

export interface MetricOut {
  label: string;
  value: number;
  unit: string;
  decimals: number;
}

export interface BestOut {
  windKw: number;
  pvKw: number;
  essKwh: number;
  essKw: number;
  fitness: number;
}

/** 逐时电量平衡序列（kWh，列口径与标准模板 output Sheet 一致） */
export interface BalanceSeriesOut {
  wind: number[];
  pv: number[];
  theoryGen: number[];
  load: number[];
  actualGen: number[];
  chargeAc: number[];
  chargeDc: number[];
  curtailed: number[];
  dischargeDc: number[];
  dischargeAc: number[];
  gridImport: number[];
  feedIn: number[];
  socDc: number[];
  endSoc: number;
}

export interface SensRowOut {
  ratio: string;
  scale: number;
  fitness: number;
  ok: boolean;
  note: string;
}

export interface SensGroupOut {
  group: string;
  element: string;
  unit: string;
  color: string;
  chartTitle: string;
  rows: SensRowOut[];
}

export interface ComputeResultPayload {
  best: BestOut;
  headline: MetricOut[];
  invest: MetricOut[];
  opex: MetricOut[];
  energyStats: MetricOut[];
  balance: BalanceSeriesOut;
  sensitivity: SensGroupOut[];
}

export interface ProgressPayload {
  progress: number;
  message: string;
}

// ---- 后端参数负载类型 ----

export interface ComputeParamsOut {
  tech: {
    dod: number;
    rate: number;
    initialSoc: number;
    chargeEff: number;
    dischargeEff: number;
    gridCapacity: number;
    avgLoadRate: number;
    selfUseGenMin: number;
    selfUseLoadMin: number;
    feedLimit: number;
    feedPower: number;
    curtailLimit: number;
  };
  econ: {
    windInvest: number;
    pvInvest: number;
    essInvest: number;
    opexRatio: number;
    salary: number;
    staffCount: number;
    discountRate: number;
    evalPeriod: number;
    otherInvest: number;
    batteryReplaceUnit: number;
    batteryReplaceRatio: number;
    batteryReplaceYear: number;
  };
  ga: {
    generations: number;
    crossoverRate: number;
    mutationRate: number;
    populationSize: number;
  };
  range: {
    windStart: number;
    windEnd: number;
    pvStart: number;
    pvEnd: number;
    essStart: number;
    essEnd: number;
  };
  scheme: string;
  objective: string;
  chargePeriods: number[];
  dischargePeriods: number[];
}

export interface CurveDataOut {
  windPu: number[];
  pvPu: number[];
  load: number[];
  price: number[];
  lossFee: number[];
  tduFee: number[];
  systemFee: number[];
  fundFee: number[];
}

function num(v: string | number, fallback = 0): number {
  const n = Number(v);
  return Number.isFinite(n) && String(v).trim() !== "" ? n : fallback;
}

/** 由全局参数表单组装后端 ComputeParams（含 GA 参数与择优范围，单位 MW/MWh） */
export function buildComputeParams(): ComputeParamsOut {
  return {
    tech: {
      dod: num(params.dod),
      rate: num(params.rate),
      initialSoc: num(params.initialSoc),
      chargeEff: num(params.chargeEff, 93),
      dischargeEff: num(params.dischargeEff, 92),
      gridCapacity: num(params.gridCapacity),
      avgLoadRate: num(params.avgLoadRate),
      selfUseGenMin: num(params.selfUseGenMin),
      selfUseLoadMin: num(params.selfUseLoadMin),
      feedLimit: num(params.feedLimit),
      feedPower: num(params.feedPower),
      curtailLimit: num(params.curtailLimit),
    },
    econ: {
      windInvest: num(params.windInvest),
      pvInvest: num(params.pvInvest),
      essInvest: num(params.essInvest),
      opexRatio: num(params.opexRatio),
      salary: num(params.salary),
      staffCount: num(params.staffCount),
      discountRate: num(params.discountRate),
      evalPeriod: num(params.evalPeriod, 15),
      otherInvest: num(params.otherInvest),
      batteryReplaceUnit: num(params.batteryReplaceUnit),
      batteryReplaceRatio: num(params.batteryReplaceRatio, 100),
      batteryReplaceYear: num(params.batteryReplaceYear, 8),
    },
    ga: {
      generations: Math.max(1, Math.round(num(params.generations, 40))),
      crossoverRate: num(params.crossoverRate, 0.5),
      mutationRate: num(params.mutationRate, 0.3),
      populationSize: Math.max(4, Math.round(num(params.populationSize, 100))),
    },
    range: {
      windStart: num(params.windStart),
      windEnd: num(params.windEnd),
      pvStart: num(params.pvStart),
      pvEnd: num(params.pvEnd),
      essStart: num(params.essStart),
      essEnd: num(params.essEnd),
    },
    scheme: params.scheme,
    objective: params.objective,
    chargePeriods: [...params.chargePeriods],
    dischargePeriods: [...params.dischargePeriods],
  };
}

/**
 * 组装后端 CurveData。
 * 优先使用上传的发电及负荷曲线文件（真实 8760h 数据）；
 * 未上传时回退为内置算例曲线（与演示数据同源）。
 */
export function buildCurveData(): CurveDataOut {
  const uploaded = activeCurve();
  if (uploaded) return uploaded;
  return {
    windPu: curveTemplate.windPu,
    pvPu: curveTemplate.pvPu,
    load: hourlyBalance.load,
    price: curveTemplate.price,
    lossFee: curveTemplate.lossFee,
    tduFee: curveTemplate.tduFee,
    systemFee: curveTemplate.systemFee,
    fundFee: curveTemplate.fundFee,
  };
}

// ---- 结果格式化 ----

/** 按指定小数位 + 千分位格式化数值 */
export function fmtNum(v: number, decimals: number): string {
  if (!Number.isFinite(v)) return "--";
  return v.toLocaleString("zh-CN", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

/** 后端指标 → 界面指标项（"91,775.18 万元"），与导出解析口径一致 */
export function formatMetricItem(m: MetricOut): { label: string; value: string } {
  return {
    label: m.label,
    value: `${fmtNum(m.value, m.decimals)}${m.unit ? " " + m.unit : ""}`,
  };
}
