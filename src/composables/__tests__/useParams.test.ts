import { describe, it, expect, beforeEach } from "vitest";
import { params, validateParams } from "@/composables/useParams";

describe("validateParams 参数合理性校验（FR-5）", () => {
  beforeEach(() => {
    // 合法基线
    params.curveFile = "curvetemplate.xlsx";
    params.dod = "80";
    params.initialSoc = "20";
    params.chargeEff = "93";
    params.dischargeEff = "92";
    params.avgLoadRate = "50";
    params.selfUseGenMin = "60";
    params.selfUseLoadMin = "30";
    params.feedLimit = "20";
    params.curtailLimit = "20";
    params.opexRatio = "2";
    params.batteryReplaceRatio = "100";
    params.crossoverRate = "0.5";
    params.mutationRate = "0.3";
    params.windStart = "0";
    params.windEnd = "200";
  });

  it("合法参数通过校验", () => {
    expect(validateParams()).toEqual([]);
  });

  it("曲线未上传必选校验失败", () => {
    params.curveFile = "";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "curveFile")).toBe(true);
  });

  it("百分比越界校验失败", () => {
    params.dod = "150";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "dod")).toBe(true);
  });

  it("DOD + 初始电量 < 100 校验失败", () => {
    params.dod = "70";
    params.initialSoc = "10";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "initialSoc")).toBe(true);
  });

  it("GA 概率越界校验失败", () => {
    params.crossoverRate = "1.5";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "crossoverRate")).toBe(true);
  });

  it("择优范围 起始 > 结束 校验失败", () => {
    params.windStart = "300";
    params.windEnd = "100";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "windStart")).toBe(true);
  });
});
