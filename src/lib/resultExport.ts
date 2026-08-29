import * as XLSX from "xlsx";
import { invoke } from "@tauri-apps/api/core";
import {
  arrayBufferToBase64,
  isTauri,
  revealSavedFile,
} from "./templates";
import { params } from "@/composables/useParams";
import { result } from "@/composables/useComputation";
import { uploadedCurveBytes } from "@/composables/useUploadedFiles";
import { curveTemplate } from "@/data/curveTemplate";
import { hourlyBalance } from "@/data/hourlyBalance";
import {
  energyStats,
  fullBalanceSeries,
  hourLabels,
  sensitivityGroups,
} from "@/composables/useResultData";

/** 输入参数导出清单（与后端结果文件"输入数据"表口径一致） */
const INPUT_ITEMS: Array<{ label: string; unit: string; key: keyof typeof params }> = [
  { label: "储能充放电深度", unit: "%", key: "dod" },
  { label: "电池充放电倍率", unit: "", key: "rate" },
  { label: "储能初始电量", unit: "%", key: "initialSoc" },
  { label: "储能系统充电效率", unit: "%", key: "chargeEff" },
  { label: "储能系统放电效率", unit: "%", key: "dischargeEff" },
  { label: "接入公共电网容量（最大下网功率）", unit: "kW", key: "gridCapacity" },
  { label: "平均负荷率", unit: "%", key: "avgLoadRate" },
  { label: "自发自用占总可用发电量比例下限", unit: "%", key: "selfUseGenMin" },
  { label: "自发自用占总用电量比例下限", unit: "%", key: "selfUseLoadMin" },
  { label: "余电上网比例上限", unit: "%", key: "feedLimit" },
  { label: "余电最大上网功率", unit: "kW", key: "feedPower" },
  { label: "弃电率上限", unit: "%", key: "curtailLimit" },
  { label: "风电系统单位投资", unit: "元/kW", key: "windInvest" },
  { label: "光伏系统单位投资", unit: "元/kW", key: "pvInvest" },
  { label: "储能系统单位投资", unit: "元/kWh", key: "essInvest" },
  { label: "年运维费用占比", unit: "%", key: "opexRatio" },
  { label: "人员工资", unit: "万元/人年", key: "salary" },
  { label: "定员人数", unit: "人", key: "staffCount" },
  { label: "折现率", unit: "%", key: "discountRate" },
  { label: "评价周期", unit: "年", key: "evalPeriod" },
  { label: "其他固定投资", unit: "万元", key: "otherInvest" },
  { label: "电池更换单价", unit: "元/kWh", key: "batteryReplaceUnit" },
  { label: "电池更换比例", unit: "%", key: "batteryReplaceRatio" },
  { label: "电池更换时间", unit: "年底", key: "batteryReplaceYear" },
];

/** 拆解 "91,775.18 万元" → { value: 91775.18, unit: "万元" } */
function parseValue(raw: string): { value: number | string; unit: string } {
  const m = raw.trim().match(/^([\d.,]+)\s*(.*)$/);
  if (!m) return { value: raw, unit: "" };
  const n = Number(m[1].replace(/,/g, ""));
  return { value: Number.isFinite(n) ? n : m[1], unit: m[2] || "" };
}

function setColWidths(ws: XLSX.WorkSheet, widths: number[]) {
  ws["!cols"] = widths.map((w) => ({ wch: w }));
}

/** 输出数据备注（计算口径与标准模板 output Sheet 一致） */
const OUTPUT_NOTES: Record<string, string> = {
  "全年储能供电量（交流侧）": "储能供电量=储能放电量×储能放电效率",
  "全年负荷用电量":
    "负荷用电量+储能充电量+余电上网电量=新能源实际发电量+储能供电量+下网电量",
  "年电网购电成本":
    "方案一：电量电价+上网环节线损费+系统运行费+政府性基金及附加+接入公共电网容量×平均负荷率×8760×电度输配电价；方案二：电量电价+上网环节线损费+电度输配电费+系统运行费+政府性基金及附加",
  "年自发自用输配成本": "方案一：政府性基金及附加；方案二：输配电费+政府性基金及附加",
};

/** 输入数据 Sheet（24 项） */
function buildInputSheet(): XLSX.WorkSheet {
  const aoa: (string | number)[][] = [
    ["输入数据"],
    ["序号", "项目", "单位", "数据", "备注"],
  ];
  INPUT_ITEMS.forEach((item, i) => {
    const raw = params[item.key] as string | number;
    const n = Number(raw);
    aoa.push([i + 1, item.label, item.unit, Number.isFinite(n) && String(raw).trim() !== "" ? n : String(raw), ""]);
  });
  const ws = XLSX.utils.aoa_to_sheet(aoa);
  setColWidths(ws, [6, 36, 12, 14, 14]);
  return ws;
}

/** 通用指标分区写入（序号 / 项目 / 单位 / 数据 / 备注） */
function pushMetricSection(
  aoa: (string | number)[][],
  title: string,
  items: readonly { label: string; value: string }[],
  notes?: Record<string, string>,
) {
  aoa.push([]);
  aoa.push([title]);
  aoa.push(["序号", "项目", "单位", "数据", "备注"]);
  items.forEach((item, i) => {
    const { value, unit } = parseValue(item.value);
    aoa.push([i + 1, item.label, unit, value, notes?.[item.label] ?? ""]);
  });
}

/** 输出数据 Sheet（最优配置 + 全年电量指标 + 投资构成 + 年运行成本构成） */
function buildOutputSheet(): XLSX.WorkSheet {
  const aoa: (string | number)[][] = [];

  pushMetricSection(aoa, "一、最优配置结果", result.headline);
  pushMetricSection(aoa, "二、全年电量指标", energyStats, OUTPUT_NOTES);
  pushMetricSection(aoa, "三、投资构成", result.invest);
  pushMetricSection(aoa, "四、年运行成本构成", result.opex, OUTPUT_NOTES);

  const ws = XLSX.utils.aoa_to_sheet(aoa);
  setColWidths(ws, [6, 32, 12, 16, 40]);
  return ws;
}

/** 敏感性分析 Sheet（固定两要素 · 变动单一要素，±25%） */
function buildSensitivitySheet(): XLSX.WorkSheet {
  const aoa: (string | number)[][] = [];
  for (const g of sensitivityGroups) {
    aoa.push([g.group]);
    aoa.push(["变动比例", `规模（${g.unit}）`, "适应度", "备注"]);
    for (const r of g.rows) {
      aoa.push([r.ratio, Number(r.scale.replace(/,/g, "")), Number(r.fitness), r.note]);
    }
    aoa.push([]);
  }
  const ws = XLSX.utils.aoa_to_sheet(aoa);
  setColWidths(ws, [12, 16, 12, 16]);
  return ws;
}

/** 逐时电量平衡 Sheet（全年 8760h，单位 kWh，列口径与标准模板一致） */
function buildBalanceSheet(): XLSX.WorkSheet {
  const aoa: (string | number)[][] = [
    ["逐时电量平衡（kWh，全年 8760h）"],
    ["小时", ...fullBalanceSeries.map((s) => s.name), "备注"],
  ];
  hourLabels.forEach((t, i) => {
    aoa.push([Number(t), ...fullBalanceSeries.map((s) => s.values[i]), ""]);
  });
  aoa.push([]);
  aoa.push([
    "说明：逐时数据取自曲线模板算例；新能源理论/实际发电量、直流侧电量与储能可用电量由交流侧电量及效率参数推算；接入真实计算接口后由后端结果替换。",
  ]);
  const ws = XLSX.utils.aoa_to_sheet(aoa);
  setColWidths(ws, [10, ...fullBalanceSeries.map(() => 16), 12]);
  return ws;
}

/** 输入曲线 Sheet 时间标签（2020 年、跳过 2 月 29 日，共 365 天 8760h，与曲线模板一致） */
const CURVE_MONTH_DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

function curveTimeLabel(i: number): string {
  const p = (n: number) => String(n).padStart(2, "0");
  let day = Math.floor(i / 24);
  const hour = i % 24;
  let m = 0;
  while (day >= CURVE_MONTH_DAYS[m]) {
    day -= CURVE_MONTH_DAYS[m];
    m++;
  }
  return `2020-${p(m + 1)}-${p(day + 1)} ${p(hour)}:00:00`;
}

/** 输入曲线 Sheet（曲线模板 curveldzl3：负荷 / 风光出力标幺值 / 分项电价曲线） */
function buildCurveSheet(): XLSX.WorkSheet {
  const headers = [
    "时间",
    "用电负荷\n（kWh）",
    "风电发电量标幺值（kWh）",
    "光伏发电量标幺值（kWh）",
    "电力现货市场交易电量电价\n（元/kWh）",
    "上网环节线损费用\n（元/kWh）",
    "电度输配电价\n（元/kWh）",
    "系统运行费用\n（元/kWh）",
    "政府性基金及附加\n（元/kWh）",
  ];
  const aoa: (string | number)[][] = [headers];
  const n = curveTemplate.windPu.length;
  for (let i = 0; i < n; i++) {
    aoa.push([
      curveTimeLabel(i),
      hourlyBalance.load[i],
      curveTemplate.windPu[i],
      curveTemplate.pvPu[i],
      curveTemplate.price[i],
      curveTemplate.lossFee[i],
      curveTemplate.tduFee[i],
      curveTemplate.systemFee[i],
      curveTemplate.fundFee[i],
    ]);
  }
  const ws = XLSX.utils.aoa_to_sheet(aoa);
  setColWidths(ws, [20, 14, 22, 22, 26, 18, 16, 16, 20]);
  return ws;
}

function timestamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}` +
    `${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`
  );
}

// ---- 结果填入上传曲线文件（真实软件 output Sheet 口径） ----

/** 标准模板 output Sheet 的 34 项输出指标（label → 单位），与曲线模板行序一致 */
const TEMPLATE_OUTPUT_ITEMS: Array<{ label: string; unit: string }> = [
  { label: "最优风电规模", unit: "MW" },
  { label: "最优光伏规模", unit: "MW" },
  { label: "最优储能功率", unit: "MW" },
  { label: "最优储能容量", unit: "MWh" },
  { label: "全年风电最大发电量", unit: "MWh" },
  { label: "全年光伏最大发电量", unit: "MWh" },
  { label: "全年新能源最大发电量", unit: "MWh" },
  { label: "全年弃风弃光电量", unit: "MWh" },
  { label: "全年新能源实际发电量", unit: "MWh" },
  { label: "全年储能充电量（交流侧）", unit: "MWh" },
  { label: "全年储能实际充电量（直流侧）", unit: "MWh" },
  { label: "全年储能实际放电量（直流侧）", unit: "MWh" },
  { label: "全年储能供电量（交流侧）", unit: "MWh" },
  { label: "全年下网电量", unit: "MWh" },
  { label: "全年负荷用电量", unit: "MWh" },
  { label: "下网电量占比", unit: "%" },
  { label: "绿电比", unit: "%" },
  { label: "储能年末剩余电量", unit: "MWh" },
  { label: "风电系统投资", unit: "万元" },
  { label: "光伏系统投资", unit: "万元" },
  { label: "储能系统投资", unit: "万元" },
  { label: "其他固定投资", unit: "万元" },
  { label: "初投资", unit: "万元" },
  { label: "年电网购电成本", unit: "万元" },
  { label: "年自发自用输配成本", unit: "万元" },
  { label: "运维成本", unit: "万元" },
  { label: "人员工资", unit: "万元" },
  { label: "年运行成本", unit: "万元" },
  { label: "年余电上网收益", unit: "万元" },
  { label: "储能电池更换成本", unit: "万元" },
  { label: "评价周期内总成本", unit: "万元" },
  { label: "评价周期内平均综合电价", unit: "元/kWh" },
  { label: "评价周期内平均绿电电价", unit: "元/kWh" },
  { label: "评价周期内平均网电电价", unit: "元/kWh" },
];

/** 指标卡 label → 标准模板行 label 别名 */
const LABEL_ALIASES: Record<string, string> = {
  周期内总成本: "评价周期内总成本",
  综合电价: "评价周期内平均综合电价",
  绿电电价: "评价周期内平均绿电电价",
  网电电价: "评价周期内平均网电电价",
};

/** 当前计算结果的数值字典（label → number），供填入曲线文件 output Sheet */
function buildResultValueMap(): Record<string, number> {
  const map: Record<string, number> = {};
  const put = (label: string, raw: string) => {
    const { value } = parseValue(raw);
    if (typeof value === "number") map[label] = value;
  };
  result.headline.forEach((i) => put(i.label, i.value));
  result.invest.forEach((i) => put(i.label, i.value));
  result.opex.forEach((i) => put(i.label, i.value));
  energyStats.forEach((i) => put(i.label, i.value));
  // 界面指标 label → 标准模板行 label（如 综合电价 → 评价周期内平均综合电价）
  for (const [from, to] of Object.entries(LABEL_ALIASES)) {
    if (map[from] !== undefined && map[to] === undefined) map[to] = map[from];
  }
  return map;
}

/** 输入参数数值字典（label → number），供填入曲线文件 output Sheet */
function buildInputValueMap(): Record<string, number> {
  const map: Record<string, number> = {};
  for (const item of INPUT_ITEMS) {
    const raw = params[item.key] as string | number;
    const n = Number(raw);
    if (Number.isFinite(n) && String(raw).trim() !== "") {
      map[item.label] = n;
    }
  }
  return map;
}

/** 将数值写入指定单元格（覆盖公式，保证展示口径与界面一致） */
function setNum(ws: XLSX.WorkSheet, r: number, c: number, v: number) {
  const addr = XLSX.utils.encode_cell({ r, c });
  const cell: XLSX.CellObject = (ws[addr] as XLSX.CellObject) ?? {};
  delete cell.f;
  delete cell.w;
  cell.t = "n";
  cell.v = v;
  ws[addr] = cell;
}

const HOURLY_DATA_START_OFFSET = 2; // 表头行下空一行（初始电量公式行）后为首个数据行

/**
 * 将计算结果填入曲线模板的 output Sheet（列口径：B 序号 / C 项目 / E 单位 / F 数据）：
 * 1. 输入数据 24 项 → F 列
 * 2. 输出数据 34 项 → F 列（覆盖模板占位值）
 * 3. 逐时电量平衡 8760h → D~P 列（合计行 SUM 公式保留，打开时自动重算）
 */
export function fillTemplateOutputSheet(ws: XLSX.WorkSheet): void {
  const rows = XLSX.utils.sheet_to_json<unknown[]>(ws, {
    header: 1,
    defval: "",
  });
  const valueMap = { ...buildResultValueMap(), ...buildInputValueMap() };

  // 1/2. 按项目名称定位行并写入数据列（F = 第 6 列，索引 5）
  for (let r = 0; r < rows.length; r++) {
    const label = String(rows[r]?.[2] ?? "").trim();
    if (!label) continue;
    const v = valueMap[label];
    if (v !== undefined) setNum(ws, r, 5, v);
  }

  // 3. 逐时数据：定位「序号 / 时间」表头行，写入其后的 8760 个数据行（D~P 列，索引 3~15）
  const headerIdx = rows.findIndex(
    (r) => String(r[1] ?? "").trim() === "序号" && String(r[2] ?? "").trim() === "时间",
  );
  if (headerIdx < 0) return;

  const series = fullBalanceSeries;
  const n = Math.min(CURVE_HOURS_SIZE, ...series.map((s) => s.values.length));
  let written = 0;
  for (let r = headerIdx + HOURLY_DATA_START_OFFSET; r < rows.length && written < n; r++, written++) {
    for (let s = 0; s < series.length; s++) {
      const v = series[s].values[written];
      if (typeof v === "number" && Number.isFinite(v)) setNum(ws, r, 3 + s, v);
    }
  }
}

const CURVE_HOURS_SIZE = 8760;

/** 标准模板 output Sheet 的逐时表头（列口径与曲线模板完全一致） */
const TEMPLATE_HOURLY_HEADERS = [
  "风电发电量\n（kWh）",
  "光伏发电量\n（kWh）",
  "新能源理论发电量\n（kWh）",
  "用户负荷\n（kWh）",
  "该小时段新能源实际发电量\n（kWh）",
  "该小时段储能充电量\n（交流侧）\n（kWh）",
  "该小时段储能实际充电量（直流侧）\n（kWh）",
  "该小时段弃风弃光电量\n(kWh)",
  "该小时段储能放电量（直流侧）\n（kWh）",
  "该小时段储能对外供电量\n（交流测）\n（kWh）",
  "该小时段下网电量\n(kWh)",
  "该小时段余电上网电量\n(kWh)",
  "储能可用电量\n（直流侧）\n（kWh）",
];

/**
 * 上传文件缺少 output Sheet 时的兜底：按标准模板结构生成完整 output Sheet
 * （输入数据 24 项 + 输出数据 34 项 + 逐时 8760h 表 + 合计行）。
 */
function buildTemplateStyleOutputSheet(): XLSX.WorkSheet {
  const valueMap = { ...buildResultValueMap(), ...buildInputValueMap() };
  const aoa: (string | number)[][] = [];

  aoa.push(["输入数据"]);
  aoa.push(["序号", "项目", "单位", "数据", "备注"]);
  INPUT_ITEMS.forEach((item, i) => {
    aoa.push([i + 1, item.label, item.unit, valueMap[item.label] ?? "", ""]);
  });
  aoa.push([]);
  aoa.push(["输出数据"]);
  TEMPLATE_OUTPUT_ITEMS.forEach((item, i) => {
    aoa.push([i + 1, item.label, item.unit, valueMap[item.label] ?? "", ""]);
  });
  aoa.push([]);
  aoa.push(["序号", "时间", ...TEMPLATE_HOURLY_HEADERS, "备注"]);

  const ws = XLSX.utils.aoa_to_sheet(aoa);
  const series = fullBalanceSeries;
  const n = Math.min(CURVE_HOURS_SIZE, ...series.map((s) => s.values.length));
  const startRow = aoa.length; // aoa_to_sheet 后数据行起始（0 基）
  for (let i = 0; i < n; i++) {
    XLSX.utils.sheet_add_aoa(
      ws,
      [[i + 1, curveTimeLabel(i), ...series.map((s) => s.values[i] ?? 0), ""]],
      { origin: { r: startRow + i, c: 0 } },
    );
  }
  // 合计行（MWh）+ 表尾标签
  const totalRow = startRow + n;
  const colLetter = (idx: number) => XLSX.utils.encode_col(idx);
  XLSX.utils.sheet_add_aoa(
    ws,
    [series.map((_, idx) => `=SUM(${colLetter(3 + idx)}${startRow + 1}:${colLetter(3 + idx)}${totalRow})/1000`)],
    { origin: { r: totalRow, c: 3 } },
  );
  XLSX.utils.sheet_add_aoa(ws, [["年合计（MWh）"]], { origin: { r: totalRow, c: 1 } });
  setColWidths(ws, [10, 20, ...TEMPLATE_HOURLY_HEADERS.map(() => 16), 12]);
  return ws;
}

/** 丰富结果 Sheet 清单（与 计算结果_*.xlsx 导出口径一致） */
const RICH_SHEETS: Array<{ name: string; build: () => XLSX.WorkSheet }> = [
  { name: "输入数据", build: buildInputSheet },
  { name: "输出数据", build: buildOutputSheet },
  { name: "敏感性分析", build: buildSensitivitySheet },
  { name: "逐时电量平衡", build: buildBalanceSheet },
];

/** 在工作簿中追加（或替换）丰富结果 Sheets */
function appendRichSheets(wb: XLSX.WorkBook): void {
  for (const { name, build } of RICH_SHEETS) {
    const idx = wb.SheetNames.indexOf(name);
    if (idx >= 0) {
      wb.SheetNames.splice(idx, 1);
      delete wb.Sheets[name];
    }
    XLSX.utils.book_append_sheet(wb, build(), name);
  }
}

/**
 * 基于上传曲线文件构建结果工作簿：
 * 结果填入曲线文件副本内部（output Sheet：输入数据 / 输出数据 / 逐时 8760h / 合计行），
 * 并补充 计算结果_*.xlsx 同口径的四个丰富结果 Sheet。
 */
export function buildCurveResultWorkbook(curveBytes: ArrayBuffer): XLSX.WorkBook {
  const wb = XLSX.read(curveBytes, { type: "array" });

  const outName = wb.SheetNames.find((n) => /output/i.test(n));
  if (outName) {
    fillTemplateOutputSheet(wb.Sheets[outName]);
  } else {
    XLSX.utils.book_append_sheet(wb, buildTemplateStyleOutputSheet(), "output");
  }

  appendRichSheets(wb);

  // 剩余公式（合计行 SUM 等）在打开文件时强制重算
  const wbProps = (wb.Workbook ??= {}) as unknown as Record<string, unknown>;
  wbProps.CalcPr = { ...((wbProps.CalcPr as object) ?? {}), fullCalcOnLoad: true };
  return wb;
}

/**
 * 构建计算结果工作簿，包含：
 * 输入数据（24 项）、输入曲线（curveldzl3）、输出数据、敏感性分析、逐时电量平衡。
 */
export function buildResultWorkbook(): XLSX.WorkBook {
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, buildInputSheet(), "输入数据");
  XLSX.utils.book_append_sheet(wb, buildCurveSheet(), "输入曲线");
  XLSX.utils.book_append_sheet(wb, buildOutputSheet(), "输出数据");
  XLSX.utils.book_append_sheet(wb, buildSensitivitySheet(), "敏感性分析");
  XLSX.utils.book_append_sheet(wb, buildBalanceSheet(), "逐时电量平衡");
  return wb;
}

/**
 * 生成并下载计算结果报告 Excel。
 * - 已上传曲线文件：计算结果填写在曲线文件副本内部（output Sheet），
 *   并补充四个丰富结果 Sheet（输入数据 / 输出数据 / 敏感性分析 / 逐时电量平衡）；
 * - 未上传：退化为独立五 Sheet 报告（输入数据 / 输入曲线 / 输出数据 / 敏感性分析 / 逐时电量平衡）。
 * 返回保存文件名/路径。
 */
export async function exportResultWorkbook(): Promise<string> {
  const curveBytes = uploadedCurveBytes();
  const wb = curveBytes
    ? buildCurveResultWorkbook(curveBytes)
    : buildResultWorkbook();
  const fileName = `计算结果_${timestamp()}.xlsx`;

  if (isTauri()) {
    const data = arrayBufferToBase64(
      XLSX.write(wb, { bookType: "xlsx", type: "array" }) as ArrayBuffer,
    );
    const savedPath = await invoke<string>("save_template_file", {
      name: fileName,
      data,
    });
    await revealSavedFile(savedPath);
    return savedPath;
  }

  XLSX.writeFile(wb, fileName, { bookType: "xlsx", type: "file" });
  return fileName;
}
