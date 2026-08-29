import { readFileSync } from "node:fs";
import path from "node:path";
import * as XLSX from "xlsx";
import { describe, expect, it } from "vitest";
import {
  buildCurveResultWorkbook,
  fillTemplateOutputSheet,
} from "../resultExport";
import { hourlyBalance } from "@/data/hourlyBalance";
import { fullBalanceSeries } from "@/composables/useResultData";

const DOCS = path.resolve(__dirname, "../../../docs");

function readDocBytes(name: string): ArrayBuffer {
  const buf = readFileSync(path.join(DOCS, name));
  return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) as ArrayBuffer;
}

/** 演示结果（useComputation.demoResults）中的最优风电规模 */
const DEMO_WIND_MW = 192.73;

describe("buildCurveResultWorkbook（结果填入上传曲线文件）", () => {
  const wb = buildCurveResultWorkbook(readDocBytes("curvetemplate_ldzl_3.0.xlsx"));

  it("保留原始 curveldzl3 曲线数据", () => {
    expect(wb.SheetNames).toContain("curveldzl3");
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["curveldzl3"], {
      header: 1,
    });
    expect(rows[1][1]).toBe(64000);
    expect(rows.length).toBeGreaterThanOrEqual(8761); // 表头 + 8760h
  });

  it("包含补充的四个丰富结果 Sheet", () => {
    for (const name of ["输入数据", "输出数据", "敏感性分析", "逐时电量平衡"]) {
      expect(wb.SheetNames).toContain(name);
    }
  });

  it("output Sheet 输入数据 24 项填入 F 列", () => {
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["output"], { header: 1 });
    // 第 1 项 储能充放电深度 = 85（界面参数默认值）
    expect(rows[2][1]).toBe(1);
    expect(rows[2][2]).toBe("储能充放电深度");
    expect(rows[2][5]).toBe(85);
    // 第 24 项 电池更换时间 = 8
    expect(rows[25][5]).toBe(8);
  });

  it("output Sheet 输出数据按界面计算结果填入（覆盖模板占位值）", () => {
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["output"], { header: 1 });
    // 模板占位值 101.68 应被覆盖为演示结果最优风电规模
    expect(rows[29][2]).toBe("最优风电规模");
    expect(rows[29][5]).toBeCloseTo(DEMO_WIND_MW, 2);
    // 初投资（行 52，序号 23）
    expect(rows[51][2]).toBe("初投资");
    expect(rows[51][5]).toBeCloseTo(94770.21, 1);
    // 年电网购电成本（行 53，序号 24）
    expect(rows[52][5]).toBeCloseTo(7988.25, 1);
    // 评价周期内平均综合电价（行 61，序号 32）
    expect(rows[60][5]).toBeCloseTo(0.2353, 3);
  });

  it("output Sheet 逐时电量平衡 8760h 填入 D~P 列", () => {
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["output"], { header: 1 });
    // 表头在第 64 行（索引 63），数据从第 66 行（索引 65）开始
    expect(String(rows[63][1])).toBe("序号");
    // 第 7 小时（索引 6）：风电发电量 = 内置逐时序列值
    expect(rows[65 + 6][3]).toBeCloseTo(hourlyBalance.wind[6], 4);
    // 最后一小时（2020-12-31 23:00）
    expect(rows[65 + 8759][1]).toBe(8760);
    expect(rows[65 + 8759][3]).toBeCloseTo(hourlyBalance.wind[8759], 4);
    // 13 列序列与 fullBalanceSeries 一致（抽样负荷列）
    expect(rows[65 + 6][6]).toBeCloseTo(fullBalanceSeries[3].values[6], 4);
  });
});

describe("fillTemplateOutputSheet", () => {
  it("不改动合计行 SUM 公式（打开文件时自动重算）", () => {
    const bytes = readDocBytes("curvetemplate_ldzl_3.0.xlsx");
    const wb = XLSX.read(bytes, { type: "array" });
    fillTemplateOutputSheet(wb.Sheets["output"]);
    const totalCell = wb.Sheets["output"]["D8826"] as XLSX.CellObject;
    expect(typeof totalCell.f).toBe("string");
    expect(totalCell.f).toContain("SUM(D65:D8825)");
  });
});
