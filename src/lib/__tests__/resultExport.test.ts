import { describe, expect, it } from "vitest";
import * as XLSX from "xlsx";
import { buildResultWorkbook } from "../resultExport";

describe("buildResultWorkbook", () => {
  it("包含五个 Sheet：输入数据 / 输入曲线 / 输出数据 / 敏感性分析 / 逐时电量平衡", () => {
    const wb = buildResultWorkbook();
    expect(wb.SheetNames).toEqual([
      "输入数据",
      "输入曲线",
      "输出数据",
      "敏感性分析",
      "逐时电量平衡",
    ]);
  });

  it("输入曲线 Sheet（curveldzl3）含全年 8760 小时与电价曲线", () => {
    const wb = buildResultWorkbook();
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["输入曲线"], { header: 1 });
    expect(rows[0][0]).toBe("时间");
    expect(rows[0]).toContain("风电发电量标幺值（kWh）");
    expect(rows[0]).toContain("电力现货市场交易电量电价\n（元/kWh）");
    expect(rows).toHaveLength(8761); // 表头 + 8760h
    // 2020-01-01 06:00：风电标幺值 0.04389375 × 192725.01kW ≈ 8459.42 kWh
    expect(rows[7]).toEqual([
      "2020-01-01 06:00:00",
      64000,
      0.04389375,
      0,
      0.5130720000000001,
      0.0094,
      0.0525,
      0.0062,
      0.022425,
    ]);
    // 年末最后一小时
    expect(rows[8760][0]).toBe("2020-12-31 23:00:00");
  });

  it("输入数据 Sheet 含 24 项参数与备注列", () => {
    const wb = buildResultWorkbook();
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["输入数据"], { header: 1 });
    expect(rows[0][0]).toBe("输入数据");
    expect(rows[1]).toContain("备注");
    // 表头 + 24 行参数
    const itemRows = rows.filter((r) => typeof r[0] === "number");
    expect(itemRows).toHaveLength(24);
    expect(itemRows[0]).toContain("储能充放电深度");
    expect(itemRows[23]).toContain("电池更换时间");
  });

  it("输出数据 Sheet 含最优配置、全年电量指标、投资与成本构成四个分区", () => {
    const wb = buildResultWorkbook();
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["输出数据"], { header: 1 });
    const texts = rows.flat().map(String);
    expect(texts).toContain("一、最优配置结果");
    expect(texts).toContain("二、全年电量指标");
    expect(texts).toContain("三、投资构成");
    expect(texts).toContain("四、年运行成本构成");
    expect(texts).toContain("最优风电规模");
    expect(texts).toContain("全年新能源实际发电量");
    // 标准模板中缺少的两项直流侧指标
    expect(texts).toContain("全年储能实际充电量（直流侧）");
    expect(texts).toContain("全年储能实际放电量（直流侧）");
    expect(texts).toContain("综合电价");
    // 备注列与计算口径说明
    expect(rows[1]).toContain("备注");
    expect(texts).toContain("储能供电量=储能放电量×储能放电效率");
    expect(texts.join()).toContain("方案一：电量电价");
  });

  it("敏感性分析 Sheet 含三组 ±25% 变动数据", () => {
    const wb = buildResultWorkbook();
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["敏感性分析"], { header: 1 });
    const texts = rows.flat().map(String);
    expect(texts).toContain("固定光储 · 变动风电");
    expect(texts).toContain("固定风储 · 变动光伏");
    expect(texts).toContain("固定风光 · 变动储能");
    const ratioRows = rows.filter((r) => String(r[0]).endsWith("%"));
    expect(ratioRows).toHaveLength(33); // 3 组 × 11 档（步长 5%）
    expect(texts).toContain("弃电率超标");
  });

  it("逐时电量平衡 Sheet 含全年 8760 小时与标准模板全部电量序列", () => {
    const wb = buildResultWorkbook();
    const rows = XLSX.utils.sheet_to_json<unknown[]>(wb.Sheets["逐时电量平衡"], { header: 1 });
    expect(rows[1][0]).toBe("小时");
    expect(rows[1]).toContain("风电发电量");
    expect(rows[1]).toContain("弃风弃光电量");
    // 标准模板补充列
    expect(rows[1]).toContain("新能源理论发电量");
    expect(rows[1]).toContain("新能源实际发电量");
    expect(rows[1]).toContain("储能实际充电量（直流侧）");
    expect(rows[1]).toContain("储能放电量（直流侧）");
    expect(rows[1]).toContain("储能可用电量（直流侧）");
    expect(rows[1]).toContain("备注");
    const hourRows = rows.filter((r) => /^\d+$/.test(String(r[0])));
    expect(hourRows).toHaveLength(8760);
  });
});
