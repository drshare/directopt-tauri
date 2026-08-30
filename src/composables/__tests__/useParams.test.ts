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
    // 默认贝叶斯优化（V3.0 口径）
    params.algorithm = "bo";
    params.nIter = "100";
    params.nInit = "20";
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

  it("贝叶斯优化：总评估次数必须大于初始采样点数", () => {
    params.nIter = "20";
    params.nInit = "20";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "nIter")).toBe(true);
  });

  it("贝叶斯优化：初始随机采样点数不能小于 2", () => {
    params.nInit = "1";
    const issues = validateParams();
    expect(issues.some((i) => i.field === "nInit")).toBe(true);
  });

  it("遗传算法：概率越界校验失败（且仅在该算法下校验）", () => {
    // 贝叶斯优化口径下 GA 概率不参与校验
    params.crossoverRate = "1.5";
    expect(validateParams().some((i) => i.field === "crossoverRate")).toBe(false);

    params.algorithm = "ga";
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
