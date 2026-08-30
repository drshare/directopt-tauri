/**
 * 上传文件真实解析（FR-1 / FR-2）
 * 1. 输入文件（inputtemplate_ldzl_3.0.xlsx · input_ldzl3 Sheet）→ 参数回填
 * 2. 发电及负荷曲线（curvetemplate_ldzl_3.0.xlsx · curveldzl3 Sheet）→ 全年 8760h 时序数据
 * 解析结果供 useUploadedFiles 共享状态与后端计算使用。
 */
import * as XLSX from "xlsx";
import type { CurveDataOut } from "./computeTypes";

/** 全年小时数（2020 年、跳过 2 月 29 日） */
export const CURVE_HOURS = 8760;

/** 输入文件「项目」名称 → 前端参数键（useParams.params） */
export const INPUT_LABEL_KEYS: Record<string, string> = {
  储能充放电深度: "dod",
  电池充放电倍率: "rate",
  储能初始电量: "initialSoc",
  储能系统充电效率: "chargeEff",
  储能系统放电效率: "dischargeEff",
  "接入公共电网容量（最大下网功率）": "gridCapacity",
  平均负荷率: "avgLoadRate",
  自发自用占总可用发电量比例下限: "selfUseGenMin",
  自发自用占总用电量比例下限: "selfUseLoadMin",
  余电上网比例上限: "feedLimit",
  余电最大上网功率: "feedPower",
  弃电率上限: "curtailLimit",
  风电系统单位投资: "windInvest",
  光伏系统单位投资: "pvInvest",
  储能系统单位投资: "essInvest",
  年运维费用占比: "opexRatio",
  人员工资: "salary",
  定员人数: "staffCount",
  折现率: "discountRate",
  评价周期: "evalPeriod",
  其他固定投资: "otherInvest",
  电池更换单价: "batteryReplaceUnit",
  电池更换比例: "batteryReplaceRatio",
  电池更换时间: "batteryReplaceYear",
  选定风电规模起始值: "windStart",
  选定风电规模结束值: "windEnd",
  选定光伏规模起始值: "pvStart",
  选定光伏规模结束值: "pvEnd",
  选定储能容量起始值: "essStart",
  选定储能容量结束值: "essEnd",
  // V3.0 算法参数：贝叶斯优化口径（界面「算法参数」区仅此两项，
  // V2.2 说明书的遗传代数 / 交叉概率 / 变异概率 / 种群大小在 V3.0 已不存在）
  总评估次数: "nIter",
  初始随机采样点数: "nInit",
};

export interface InputParseResult {
  /** 参数键 → 数值 */
  values: Record<string, number>;
  /** 成功识别并回填的参数名 */
  appliedLabels: string[];
  /** 无法识别的行（不在标准模板内的项目） */
  skippedLabels: string[];
  /** 解析使用的工作表名 */
  sheetName: string;
}

function toFiniteNumber(v: unknown): number | null {
  if (v === null || v === undefined || typeof v === "boolean") return null;
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  const s = String(v).trim().replace(/,/g, "");
  if (s === "") return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

/**
 * 解析输入文件（input_ldzl3 等），提取「项目 / 数值」参数。
 * 结构：表头行含「项目」与「数值」列，其后每行一项。
 */
export function parseInputWorkbook(data: ArrayBuffer): InputParseResult {
  const wb = XLSX.read(data, { type: "array" });
  if (wb.SheetNames.length === 0) {
    throw new Error("输入文件中没有工作表");
  }
  const sheetName =
    wb.SheetNames.find((n) => /input/i.test(n)) ?? wb.SheetNames[0];
  const ws = wb.Sheets[sheetName];
  const rows = XLSX.utils.sheet_to_json<unknown[]>(ws, {
    header: 1,
    defval: "",
    blankrows: false,
  });

  const headerIdx = rows.findIndex((r) => {
    const cells = r.map((c) => String(c ?? "").trim());
    return cells.includes("项目") && cells.includes("数值");
  });
  if (headerIdx < 0) {
    throw new Error(
      `工作表「${sheetName}」中未找到「项目 / 数值」表头，请使用标准输入文件模板`,
    );
  }

  const header = rows[headerIdx].map((c) => String(c ?? "").trim());
  const labelCol = header.indexOf("项目");
  const valueCol = header.indexOf("数值");

  const values: Record<string, number> = {};
  const appliedLabels: string[] = [];
  const skippedLabels: string[] = [];

  for (const row of rows.slice(headerIdx + 1)) {
    const label = String(row[labelCol] ?? "").trim();
    if (!label) continue;
    const key = INPUT_LABEL_KEYS[label];
    const n = toFiniteNumber(row[valueCol]);
    if (key && n !== null) {
      values[key] = n;
      appliedLabels.push(label);
    } else {
      skippedLabels.push(label);
    }
  }

  if (appliedLabels.length === 0) {
    throw new Error(
      `工作表「${sheetName}」中未识别到任何标准参数，请使用标准输入文件模板`,
    );
  }

  return { values, appliedLabels, skippedLabels, sheetName };
}

export interface CurveParseResult {
  /** 全年 8760h 时序曲线（与后端 CurveData 口径一致） */
  curve: CurveDataOut;
  /** 实际解析到的数据行数 */
  rowCount: number;
  /** 非致命提示（如数据行超出 8760h 已截断） */
  warnings: string[];
  /** 解析使用的工作表名 */
  sheetName: string;
}

const CURVE_COLUMNS: Array<{ key: keyof CurveDataOut; label: string }> = [
  { key: "load", label: "用电负荷" },
  { key: "windPu", label: "风电发电量标幺值" },
  { key: "pvPu", label: "光伏发电量标幺值" },
  { key: "price", label: "电量电价" },
  { key: "lossFee", label: "上网环节线损费用" },
  { key: "tduFee", label: "电度输配电价" },
  { key: "systemFee", label: "系统运行费用" },
  { key: "fundFee", label: "政府性基金及附加" },
];

/**
 * 解析发电及负荷曲线文件（curveldzl3 等）。
 * 结构：表头行首列为「时间」，其后 8 列依次为负荷 / 风光标幺值 / 电价与三项费用。
 * 要求不少于 8760 行数据（2020 年全年，超出部分截断）。
 */
export function parseCurveWorkbook(data: ArrayBuffer): CurveParseResult {
  const wb = XLSX.read(data, { type: "array" });
  if (wb.SheetNames.length === 0) {
    throw new Error("曲线文件中没有工作表");
  }
  const sheetName =
    wb.SheetNames.find((n) => /curve/i.test(n)) ?? wb.SheetNames[0];
  const ws = wb.Sheets[sheetName];
  const rows = XLSX.utils.sheet_to_json<unknown[]>(ws, {
    header: 1,
    defval: null,
    blankrows: false,
  });

  const headerIdx = rows.findIndex(
    (r) => String(r[0] ?? "").trim() === "时间",
  );
  if (headerIdx < 0) {
    throw new Error(
      `工作表「${sheetName}」中未找到「时间」表头，请使用标准曲线模板`,
    );
  }

  const dataRows = rows.slice(headerIdx + 1).filter((r) => {
    // 数据行：时间列非空或任一数据列非空
    return r.some((c) => c !== null && c !== undefined && String(c).trim() !== "");
  });

  if (dataRows.length < CURVE_HOURS) {
    throw new Error(
      `曲线数据不足：共 ${dataRows.length} 行，全年 8760h 至少需要 ${CURVE_HOURS} 行，请检查文件内容`,
    );
  }

  const used = dataRows.slice(0, CURVE_HOURS);
  const warnings: string[] = [];
  if (dataRows.length > CURVE_HOURS) {
    warnings.push(`曲线共 ${dataRows.length} 行，已取前 ${CURVE_HOURS} 小时数据`);
  }

  const curve = {} as CurveDataOut;
  CURVE_COLUMNS.forEach(({ key, label }, colIdx) => {
    const series: number[] = new Array(CURVE_HOURS);
    let missing = 0;
    for (let i = 0; i < CURVE_HOURS; i++) {
      const n = toFiniteNumber(used[i]?.[colIdx + 1]);
      if (n === null) {
        missing++;
        series[i] = 0;
      } else {
        series[i] = n;
      }
    }
    if (missing > 0) {
      warnings.push(`「${label}」列有 ${missing} 个非数值单元格，已按 0 处理`);
    }
    curve[key] = series;
  });

  return { curve, rowCount: CURVE_HOURS, warnings, sheetName };
}
