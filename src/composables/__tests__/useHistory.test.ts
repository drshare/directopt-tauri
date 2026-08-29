import { beforeEach, describe, expect, it } from "vitest";
import { addHistory, clearHistory, history, removeHistory } from "../useHistory";
import type { ResultBundle } from "@/composables/useComputation";

const fakeResult: ResultBundle = {
  headline: [{ label: "最优风电规模", value: "192.73 MW" }],
  invest: [{ label: "初投资", value: "94,770.21 万元" }],
  opex: [{ label: "年运行成本", value: "10,239.39 万元" }],
};

describe("useHistory 计算历史", () => {
  beforeEach(() => {
    clearHistory();
  });

  it("新增记录：最新在前，并携带参数与结果快照", () => {
    addHistory({
      curveFile: "curve.xlsx",
      schemeLabel: "方案一",
      objectiveLabel: "综合电价最低",
      params: { generations: "40" },
      result: fakeResult,
    });
    expect(history).toHaveLength(1);
    expect(history[0].curveFile).toBe("curve.xlsx");
    expect(history[0].schemeLabel).toBe("方案一");
    expect(history[0].params.generations).toBe("40");
    expect(history[0].result.headline[0].value).toBe("192.73 MW");
    expect(history[0].atLabel).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
  });

  it("删除单条记录", () => {
    const first = addHistory({
      curveFile: "a.xlsx",
      schemeLabel: "方案一",
      objectiveLabel: "综合电价最低",
      params: {},
      result: fakeResult,
    });
    addHistory({
      curveFile: "b.xlsx",
      schemeLabel: "方案二",
      objectiveLabel: "绿电电价最低",
      params: {},
      result: fakeResult,
    });
    removeHistory(first.id);
    expect(history).toHaveLength(1);
    expect(history[0].curveFile).toBe("b.xlsx");
  });

  it("清空全部记录", () => {
    addHistory({ curveFile: "a.xlsx", schemeLabel: "方案一", objectiveLabel: "综合电价最低", params: {}, result: fakeResult });
    addHistory({ curveFile: "b.xlsx", schemeLabel: "方案二", objectiveLabel: "绿电电价最低", params: {}, result: fakeResult });
    clearHistory();
    expect(history).toHaveLength(0);
  });
});
