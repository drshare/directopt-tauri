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

  // 📋 用户输入参数快照（计算前入参总览，点击条目展开查看完整参数）
  addLog(
    "开始计算",
    "info",
    "📋 用户输入参数快照（计算前入参总览）",
    jsonDetail({
      寻优算法: params.algorithm === "ga" ? "遗传算法（V2.2 口径）" : "贝叶斯优化（V3.0 口径）",
      输配电费方案: SCHEME_LABELS[params.scheme],
      优化目标: OBJECTIVE_LABELS[params.objective],
      曲线文件: params.curveFile || "内置算例",
      输入文件: params.inputFile || "（无）",
      择优范围: {
        风电: `${params.windStart} ~ ${params.windEnd} MW`,
        光伏: `${params.pvStart} ~ ${params.pvEnd} MW`,
        储能: `${params.essStart} ~ ${params.essEnd} MWh`,
      },
      算法参数:
        params.algorithm === "ga"
          ? {
              种群大小: params.populationSize,
              遗传代数: params.generations,
              交叉概率: params.crossoverRate,
              变异概率: params.mutationRate,
            }
          : {
              总评估次数: params.nIter,
              初始随机采样点数: params.nInit,
            },
      技术参数: {
        储能充放电深度: `${params.dod}%`,
        电池充放电倍率: params.rate,
        储能初始电量: `${params.initialSoc}%`,
        充电效率: `${params.chargeEff}%`,
        放电效率: `${params.dischargeEff}%`,
        接入电网容量: `${params.gridCapacity} kW`,
        平均负荷率: `${params.avgLoadRate}%`,
        自发自用下限: `占总发电 ${params.selfUseGenMin}% / 占总用电 ${params.selfUseLoadMin}%`,
        余电上网上限: `${params.feedLimit}%（最大功率 ${params.feedPower} kW）`,
        弃电率上限: `${params.curtailLimit}%`,
      },
      经济参数: {
        风电投资: `${params.windInvest} 元/kW`,
        光伏投资: `${params.pvInvest} 元/kW`,
        储能投资: `${params.essInvest} 元/kWh`,
        运维占比: `${params.opexRatio}%`,
        人员配置: `${params.salary} 万元/人 × ${params.staffCount} 人`,
        折现率: `${params.discountRate}%`,
        评价周期: `${params.evalPeriod} 年`,
        其他固定投资: `${params.otherInvest} 万元`,
        电池更换: `${params.batteryReplaceUnit} 元/kWh × ${params.batteryReplaceRatio}% @ 第 ${params.batteryReplaceYear} 年末`,
      },
    }),
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
