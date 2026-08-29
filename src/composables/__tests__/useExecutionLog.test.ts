import { beforeEach, describe, expect, it } from "vitest";
import {
  addLog,
  beginRun,
  clearLogs,
  endRun,
  executionLogs,
  jsonDetail,
} from "../useExecutionLog";
import { addHistory, clearHistory, history } from "../useHistory";

describe("useExecutionLog", () => {
  beforeEach(() => {
    clearHistory();
    clearLogs();
  });

  it("记录日志条目（时间/阶段/级别/详情）", () => {
    addLog("文件解析", "info", "开始解析输入文件");
    addLog("文件解析", "success", "解析成功", jsonDetail({ 回填参数: 30 }));

    expect(executionLogs.logs).toHaveLength(2);
    const [first, second] = executionLogs.logs;
    expect(first.stage).toBe("文件解析");
    expect(first.level).toBe("info");
    expect(first.message).toContain("输入文件");
    expect(first.time).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    expect(second.detail).toContain("回填参数");
  });

  it("beginRun / endRun 捕获一次计算运行的全部日志", () => {
    addLog("文件解析", "info", "运行外的日志不应计入本次运行");
    beginRun();
    addLog("开始计算", "info", "提交计算任务");
    addLog("计算进度", "info", "[5%] 遗传算法进化中：第 1/40 代");
    addLog("计算完成", "success", "计算完成，耗时 12.3 秒");
    const runLogs = endRun();

    expect(runLogs).toHaveLength(3);
    expect(runLogs[0].stage).toBe("开始计算");
    expect(runLogs[2].stage).toBe("计算完成");
    // 运行结束后新增日志不再计入
    addLog("结果导出", "info", "生成结果文件");
    expect(endRun()).toHaveLength(0);
  });

  it("日志随历史记录一起保存", () => {
    beginRun();
    addLog("文件解析", "success", "输入文件解析成功：识别 30 项参数");
    addLog("开始计算", "info", "提交计算任务至后端");
    addLog("计算完成", "success", "计算完成");
    const logs = endRun();

    addHistory({
      curveFile: "curvetemplate_ldzl_3.0.xlsx",
      schemeLabel: "方案一",
      objectiveLabel: "综合电价最低",
      params: { generations: "40" },
      result: { headline: [], invest: [], opex: [] },
      logs,
    });

    expect(history).toHaveLength(1);
    expect(history[0].logs).toHaveLength(3);
    expect(history[0].logs?.map((l) => l.stage)).toEqual([
      "文件解析",
      "开始计算",
      "计算完成",
    ]);
  });

  it("未保存进历史的运行结束不产生残留", () => {
    beginRun();
    addLog("计算进度", "warn", "计算已被用户取消");
    const logs = endRun();
    expect(logs).toHaveLength(1);
    expect(endRun()).toHaveLength(0);
  });
});
