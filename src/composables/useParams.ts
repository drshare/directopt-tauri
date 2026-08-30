import { reactive } from "vue";

/**
 * 全局参数状态（FR-4 参数确认与调整）
 * 由各参数表单组件读写；上传文件读取后回填（界面修改覆盖上传值）
 */
export const params = reactive({
  // ---- 技术参数（DR-1.1）----
  dod: "85", // 储能充放电深度 %
  rate: "0.5", // 电池充放电倍率 C
  initialSoc: "20", // 储能初始电量 %
  chargeEff: "93", // 储能充电效率 %
  dischargeEff: "92", // 储能放电效率 %
  gridCapacity: "80000", // 接入公共电网容量（最大下网功率）kW
  avgLoadRate: "50", // 平均负荷率 %
  selfUseGenMin: "60", // 自发自用占总可用发电量比例下限 %
  selfUseLoadMin: "30", // 自发自用占总用电量比例下限 %
  feedLimit: "20", // 余电上网比例上限 %
  feedPower: "80000", // 余电最大上网功率 kW
  curtailLimit: "20", // 弃电率上限 %

  // ---- 经济评价参数（DR-1.2）----
  windInvest: "3600", // 风电系统单位投资 元/kW
  pvInvest: "2700", // 光伏系统单位投资 元/kW
  essInvest: "600", // 储能系统单位投资 元/kWh
  opexRatio: "1", // 年运维费用占比 %
  salary: "10", // 人员工资 万元/人年
  staffCount: "10", // 定员人数 人
  discountRate: "3", // 折现率 %
  evalPeriod: "15", // 评价周期 年
  otherInvest: "1500", // 其他固定投资 万元
  batteryReplaceUnit: "400", // 电池更换单价 元/kWh
  batteryReplaceRatio: "100", // 电池更换比例 %
  batteryReplaceYear: "8", // 电池更换时间 年末

  // ---- 择优范围（DR-1.3）----
  windStart: "0",
  windEnd: "200",
  pvStart: "0",
  pvEnd: "200",
  essStart: "0",
  essEnd: "300",

  // ---- 寻优算法（V3.0 默认贝叶斯优化，可选 V2.2 遗传算法）----
  algorithm: "bo" as "bo" | "ga",
  // 贝叶斯优化参数（V3.0 界面「算法参数」区，与 inputtemplate 的
  // 「总评估次数」「初始随机采样点数」一一对应）
  nIter: "100",
  nInit: "20",
  // 遗传算法参数（V2.2 说明书口径，仅 algorithm="ga" 时生效）
  generations: "40",
  crossoverRate: "0.5",
  mutationRate: "0.3",
  populationSize: "100",

  // ---- 储能时段（仅界面操作）----
  chargePeriods: [] as number[],
  dischargePeriods: [] as number[],

  // ---- 方案与目标（仅界面操作）----
  scheme: "scheme1" as "scheme1" | "scheme2",
  objective: "composite" as "composite" | "green" | "capex",

  // ---- 文件上传 ----
  inputFile: "",
  curveFile: "",
});

/** 输配电费方案 / 优化目标 → 展示文案 */
export const SCHEME_LABELS = {
  scheme1: "方案一",
  scheme2: "方案二",
} as const;

export const OBJECTIVE_LABELS = {
  composite: "综合电价最低",
  green: "绿电电价最低",
  capex: "初投资最低",
} as const;

export interface ParamIssue {
  field: string;
  label: string;
  message: string;
}

/** 深拷贝一份参数快照（数组字段复制引用，避免历史记录随界面改动） */
export function snapshotParams(): Record<string, string | number | number[]> {
  return {
    ...params,
    chargePeriods: [...params.chargePeriods],
    dischargePeriods: [...params.dischargePeriods],
  };
}

/** 取数值；非数值返回 NaN */
function num(v: string): number {
  const n = Number(v);
  return Number.isFinite(n) ? n : Number.NaN;
}

/**
 * 参数合理性校验（FR-5）
 * 校验点：必填（曲线必传）、百分比类 0~100、DOD+初始电量≥100、GA 概率 0~1、择优范围起≤止
 */
export function validateParams(): ParamIssue[] {
  const issues: ParamIssue[] = [];

  // 曲线必传
  if (!params.curveFile) {
    issues.push({ field: "curveFile", label: "发电及负荷曲线", message: "发电及负荷曲线为必选，请上传曲线模板文件" });
  }

  // 百分比类 0~100
  const pctFields: Array<[keyof typeof params, string]> = [
    ["dod", "储能充放电深度"],
    ["initialSoc", "储能初始电量"],
    ["chargeEff", "储能充电效率"],
    ["dischargeEff", "储能放电效率"],
    ["avgLoadRate", "平均负荷率"],
    ["selfUseGenMin", "自发自用占总可用发电量比例下限"],
    ["selfUseLoadMin", "自发自用占总用电量比例下限"],
    ["feedLimit", "余电上网比例上限"],
    ["curtailLimit", "弃电率上限"],
    ["opexRatio", "年运维费用占比"],
    ["batteryReplaceRatio", "电池更换比例"],
  ];
  for (const [k, label] of pctFields) {
    const n = num(String(params[k]));
    if (Number.isNaN(n)) issues.push({ field: k, label, message: `${label} 必须为数值` });
    else if (n < 0 || n > 100) issues.push({ field: k, label, message: `${label} 需在 0~100 之间` });
  }

  // DOD + 储能初始电量 ≥ 100
  const dod = num(params.dod);
  const soc = num(params.initialSoc);
  if (Number.isFinite(dod) && Number.isFinite(soc) && dod + soc < 100) {
    issues.push({ field: "initialSoc", label: "储能初始电量", message: "储能初始电量 + 充放电深度 ≥ 100%" });
  }

  // 按所选算法校验对应参数（另一套参数不参与校验）
  if (params.algorithm === "ga") {
    // GA 概率 0~1
    const cross = num(params.crossoverRate);
    const mut = num(params.mutationRate);
    if (cross < 0 || cross > 1) issues.push({ field: "crossoverRate", label: "交叉概率", message: "交叉概率需在 0~1 之间" });
    if (mut < 0 || mut > 1) issues.push({ field: "mutationRate", label: "变异概率", message: "变异概率需在 0~1 之间" });
    if (num(params.populationSize) < 4) issues.push({ field: "populationSize", label: "种群大小", message: "种群大小不能小于 4" });
    if (num(params.generations) < 1) issues.push({ field: "generations", label: "遗传代数", message: "遗传代数不能小于 1" });
  } else {
    // 贝叶斯优化（V3.0 默认）
    const nInit = num(params.nInit);
    const nIter = num(params.nIter);
    if (nInit < 2) issues.push({ field: "nInit", label: "初始随机采样点数", message: "初始随机采样点数不能小于 2" });
    if (nIter <= nInit) issues.push({ field: "nIter", label: "总评估次数", message: "总评估次数必须大于初始随机采样点数" });
  }

  // 择优范围 起 ≤ 止
  const rangePairs: Array<[keyof typeof params, keyof typeof params, string]> = [
    ["windStart", "windEnd", "风电"],
    ["pvStart", "pvEnd", "光伏"],
    ["essStart", "essEnd", "储能"],
  ];
  for (const [s, e, label] of rangePairs) {
    const ns = num(String(params[s]));
    const ne = num(String(params[e]));
    if (Number.isFinite(ns) && Number.isFinite(ne) && ns > ne) {
      issues.push({ field: s, label: `${label}规模`, message: `${label}规模起始值不能大于结束值` });
    }
  }

  return issues;
}
