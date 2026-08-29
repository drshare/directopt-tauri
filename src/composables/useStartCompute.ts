import { ref } from "vue";
import { computation, result, runComputation } from "@/composables/useComputation";
import {
  OBJECTIVE_LABELS,
  SCHEME_LABELS,
  params,
  snapshotParams,
  validateParams,
  type ParamIssue,
} from "@/composables/useParams";
import { addHistory } from "@/composables/useHistory";
import {
  addLog,
  beginRun,
  endRun,
  jsonDetail,
} from "@/composables/useExecutionLog";

/** FR-5 参数校验问题列表（由 AppHeader 触发计算时写入，ComputeControl 展示） */
export const issues = ref<ParamIssue[]>([]);

/**
 * 启动一次优化计算（顶栏「开始」按钮触发）：
 * 参数校验 → 写入执行日志 → 启动后端计算 → 完成后写入历史记录。
 */
export async function startCompute(): Promise<void> {
  if (computation.status === "queued" || computation.status === "running") return;

  // 开始一次计算运行的日志（随历史记录保存）
  beginRun();

  // FR-5 参数合理性校验：不通过则阻止进入计算
  issues.value = validateParams();
  if (issues.value.length > 0) {
    computation.status = "error";
    computation.message = `参数校验未通过，共 ${issues.value.length} 项，请修正后重试`;
    addLog(
      "开始计算",
      "error",
      `参数校验未通过，共 ${issues.value.length} 项，已阻止进入计算`,
      jsonDetail({
        校验问题: issues.value.map((i) => ({ 参数: i.label, 问题: i.message })),
      }),
    );
    endRun();
    return;
  }

  addLog(
    "开始计算",
    "success",
    `参数校验通过，启动优化计算（方案：${SCHEME_LABELS[params.scheme]}，目标：${OBJECTIVE_LABELS[params.objective]}，曲线文件：${params.curveFile || "内置算例"}）`,
  );

  // 启动后端计算（浏览器环境自动回退为演示流程）
  await runComputation();

  if (computation.status === "done") {
    // 计算完成后写入历史记录（参数快照 + 结果快照 + 执行日志）
    const logs = endRun();
    addHistory({
      curveFile: params.curveFile,
      schemeLabel: SCHEME_LABELS[params.scheme],
      objectiveLabel: OBJECTIVE_LABELS[params.objective],
      params: snapshotParams(),
      result: {
        headline: result.headline.map((item) => ({ ...item })),
        invest: result.invest.map((item) => ({ ...item })),
        opex: result.opex.map((item) => ({ ...item })),
      },
      logs,
    });
  } else {
    endRun();
  }
}
