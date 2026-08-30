import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  CURVE_HOURS,
  parseCurveWorkbook,
  parseInputWorkbook,
} from "../fileParsers";

const DOCS_CANDIDATES = [
  path.resolve(__dirname, "../../../docs/example"), // 夹具所在目录
  path.resolve(__dirname, "../../../docs"),
];

function readDoc(name: string): ArrayBuffer {
  for (const dir of DOCS_CANDIDATES) {
    try {
      const buf = readFileSync(path.join(dir, name));
      return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) as ArrayBuffer;
    } catch {
      // 尝试下一个候选目录
    }
  }
  throw new Error(`找不到测试夹具文件「${name}」（已搜索 ${DOCS_CANDIDATES.join(", ")}）`);
}

describe("parseInputWorkbook（输入文件 inputtemplate_ldzl_3.0.xlsx）", () => {
  const result = parseInputWorkbook(readDoc("inputtemplate_ldzl_3.0.xlsx"));

  it("使用标准 input_ldzl3 工作表", () => {
    expect(result.sheetName).toBe("input_ldzl3");
  });

  it("识别全部 24 项技术/经济参数、6 项择优范围与 2 项 V3.0 算法参数", () => {
    // V3.0 的「总评估次数」「初始随机采样点数」为贝叶斯优化参数，已纳入映射表
    expect(result.appliedLabels).toHaveLength(32);
    expect(result.skippedLabels).toEqual([]);
    expect(result.values.nIter).toBe(100);
    expect(result.values.nInit).toBe(20);
  });

  it("参数数值与模板一致", () => {
    expect(result.values.dod).toBe(85);
    expect(result.values.rate).toBe(0.5);
    expect(result.values.initialSoc).toBe(20);
    expect(result.values.gridCapacity).toBe(80000);
    expect(result.values.feedPower).toBe(80000);
    expect(result.values.windInvest).toBe(3600);
    expect(result.values.essInvest).toBe(600);
    expect(result.values.discountRate).toBe(3);
    expect(result.values.evalPeriod).toBe(15);
    expect(result.values.batteryReplaceYear).toBe(8);
  });

  it("择优范围（MW / MWh）正确映射", () => {
    expect(result.values.windStart).toBe(1);
    expect(result.values.windEnd).toBe(500);
    expect(result.values.pvStart).toBe(1);
    expect(result.values.pvEnd).toBe(500);
    expect(result.values.essStart).toBe(1);
    expect(result.values.essEnd).toBe(500);
  });

  it("非法文件（无「项目/数值」表头）抛出可读错误", () => {
    const xlsxBytes = readDoc("curvetemplate_ldzl_3.0.xlsx");
    expect(() => parseInputWorkbook(xlsxBytes)).toThrow(/表头/);
  });
});

describe("parseCurveWorkbook（发电及负荷曲线 curvetemplate_ldzl_3.0.xlsx）", () => {
  const result = parseCurveWorkbook(readDoc("curvetemplate_ldzl_3.0.xlsx"));

  it("使用标准 curveldzl3 工作表并解析 8760h", () => {
    expect(result.sheetName).toBe("curveldzl3");
    expect(result.rowCount).toBe(CURVE_HOURS);
  });

  it("八列时序数据与模板一致", () => {
    expect(result.curve.load[0]).toBe(64000);
    expect(result.curve.windPu[6]).toBeCloseTo(0.04389375, 6);
    expect(result.curve.pvPu[9]).toBeCloseTo(0.20731, 6);
    expect(result.curve.price[0]).toBeCloseTo(0.146592, 6);
    expect(result.curve.price[9]).toBeCloseTo(0.3054, 6);
    expect(result.curve.lossFee[0]).toBeCloseTo(0.0094, 6);
    expect(result.curve.tduFee[0]).toBeCloseTo(0.0525, 6);
    expect(result.curve.systemFee[0]).toBeCloseTo(0.0062, 6);
    expect(result.curve.fundFee[0]).toBeCloseTo(0.022425, 6);
  });

  it("年末最后一小时数据完整", () => {
    expect(result.curve.load[CURVE_HOURS - 1]).toBeGreaterThan(0);
  });

  it("全部序列长度为 8760", () => {
    const c = result.curve;
    expect([c.load, c.windPu, c.pvPu, c.price, c.lossFee, c.tduFee, c.systemFee, c.fundFee].map((s) => s.length)).toEqual(
      Array(8).fill(8760),
    );
  });
});
