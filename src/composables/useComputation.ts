import { reactive, readonly } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "@/lib/templates";
import {
  buildComputeParams,
  buildCurveData,
  formatMetricItem,
  type ComputeResultPayload,
  type ProgressPayload,
} from "@/lib/computeTypes";
import { applyComputeResult } from "@/composables/useResultData";

/**
 * 计算任务共享状态
 * 由 ComputeControl 更新，ResultSection 读取，实现“计算完成 → 展示结果”联动
 */
export type ComputeStatus = "idle" | "queued" | "running" | "done" | "error";

export interface ComputationState {
  status: ComputeStatus;
  queueCount: number;
  progress: number;
  message: string;
  /** 本次计算开始时间戳（ms），用于展示耗时 */
  startedAt: number;
  /** 用户请求取消（后端将在代间检查点终止） */
  cancelRequested: boolean;
}

export const computation = reactive<ComputationState>({
  status: "idle",
  queueCount: 0,
  progress: 0,
  message: "",
  startedAt: 0,
  cancelRequested: false,
});

export const computationState = readonly(computation);

export interface MetricItem {
  label: string;
  value: string;
}

/** 单次计算结果（头部指标卡 + 投资构成 + 年运行成本构成） */
export interface ResultBundle {
  headline: MetricItem[];
  invest: MetricItem[];
  opex: MetricItem[];
}

/**
 * 演示用结果数据（来源：曲线模板 output Sheet · 新能源优化 3.0 算例）
 * 接入真实计算接口后由后端结果替换。
 */
export const demoResults: ResultBundle = {
  /** 最优配置结果（头部指标卡，15 项） */
  headline: [
    { label: "最优风电规模", value: "192.73 MW" },
    { label: "最优光伏规模", value: "78.37 MW" },
    { label: "最优储能功率", value: "22.74 MW" },
    { label: "最优储能容量", value: "45.48 MWh" },
    { label: "初投资", value: "94,770.21 万元" },
    { label: "年运行成本", value: "10,239.39 万元" },
    { label: "下网电量占总用电量比例", value: "27.53%" },
    { label: "自发自用占总用电量比例", value: "72.47%" },
    { label: "弃电率", value: "16.12%" },
    { label: "余电上网比例", value: "20.0%" },
    { label: "自发自用占总可用电量比例", value: "63.88%" },
    { label: "周期内总成本", value: "157,507.58 万元" },
    { label: "综合电价", value: "0.2353 元/kWh" },
    { label: "绿电电价", value: "0.1281 元/kWh" },
    { label: "网电电价", value: "0.5176 元/kWh" },
  ],
  /** 投资构成 */
  invest: [
    { label: "风电系统投资", value: "69,381.00 万元" },
    { label: "光伏系统投资", value: "21,160.58 万元" },
    { label: "储能系统投资", value: "2,728.63 万元" },
    { label: "其他固定投资", value: "1,500.00 万元" },
    { label: "初投资", value: "94,770.21 万元" },
  ],
  /** 年运行成本构成 */
  opex: [
    { label: "年电网购电成本", value: "7,988.25 万元" },
    { label: "年自发自用输配成本", value: "1,203.44 万元" },
    { label: "运维成本", value: "947.70 万元" },
    { label: "人员工资", value: "100.00 万元" },
    { label: "年余电上网收益", value: "5,104.39 万元" },
    { label: "储能电池更换成本", value: "1,436.01 万元" },
    { label: "年运行成本", value: "10,239.39 万元" },
  ],
};

/** 深拷贝指标列表，避免响应式/历史记录之间共享引用 */
function cloneMetrics(items: readonly MetricItem[]): MetricItem[] {
  return items.map((item) => ({ ...item }));
}

/** 当前展示的计算结果（可被历史记录“载入”替换） */
export const result = reactive<ResultBundle>({
  headline: cloneMetrics(demoResults.headline),
  invest: cloneMetrics(demoResults.invest),
  opex: cloneMetrics(demoResults.opex),
});

/** 用历史记录结果替换当前展示结果 */
export function restoreResult(bundle: ResultBundle): void {
  result.headline = cloneMetrics(bundle.headline);
  result.invest = cloneMetrics(bundle.invest);
  result.opex = cloneMetrics(bundle.opex);
}

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/** 请求取消当前计算（仅 Tauri 环境生效，后端在代间检查点安全终止） */
export async function requestCancel(): Promise<void> {
  if (computation.status !== "queued" && computation.status !== "running") return;
  computation.cancelRequested = true;
  if (isTauri()) {
    try {
      await invoke("cancel_compute");
    } catch {
      // 忽略取消指令发送失败（任务可能已结束）
    }
  } else {
    // 浏览器演示流程：直接复位状态
    computation.status = "idle";
    computation.message = "计算已取消";
    computation.cancelRequested = false;
  }
}

/**
 * 演示计算流程：排队 → 计算中（进度递增）→ 完成。
 * 仅在浏览器（非 Tauri）环境下作为演示回退使用。
 */
export async function simulateComputation(): Promise<void> {
  computation.status = "queued";
  computation.queueCount = 1;
  computation.cancelRequested = false;
  computation.startedAt = Date.now();
  computation.message = "任务等待中…当前队列共有 1 个任务等待执行";
  await delay(1500);

  computation.status = "running";
  computation.progress = 0;
  computation.message = "努力计算中…请勿关闭页面，否则无法显示计算结果";
  for (let progress = 0; progress <= 100; progress += 10) {
    computation.progress = progress;
    await delay(120);
  }

  computation.status = "done";
  computation.message = "计算完成，可在下方查看结果并导出报告";
}

/**
 * 真实计算流程（全部计算在后端 Rust 执行，确保准确性）：
 * 1. 组装参数与曲线，invoke("start_compute") 提交后端任务；
 * 2. 监听 compute://progress 事件更新进度（遗传算法逐代回报）；
 * 3. 命令返回后用后端结果负载替换当前展示结果与逐时/敏感性数据；
 * 4. 浏览器（非 Tauri）环境回退为演示流程。
 * 状态机与消息口径保持不变。
 */
export async function runComputation(): Promise<void> {
  if (!isTauri()) {
    await simulateComputation();
    return;
  }

  computation.status = "queued";
  computation.queueCount = 1;
  computation.progress = 0;
  computation.cancelRequested = false;
  computation.startedAt = Date.now();
  computation.message = "任务已提交至后端计算引擎，正在排队执行…";

  let unlisten: (() => void) | null = null;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<ProgressPayload>("compute://progress", (e) => {
      computation.status = "running";
      computation.progress = Math.max(0, Math.min(100, Math.round(e.payload.progress)));
      computation.message = e.payload.message;
    });

    const payload = await invoke<ComputeResultPayload>("start_compute", {
      params: buildComputeParams(),
      curves: buildCurveData(),
    });

    // 后端结果 → 界面数据源（逐时电量平衡 / 全年电量指标 / 敏感性分析）
    applyComputeResult(payload);
    // 后端指标 → 头部/投资/成本指标卡
    result.headline = payload.headline.map(formatMetricItem);
    result.invest = payload.invest.map(formatMetricItem);
    result.opex = payload.opex.map(formatMetricItem);

    computation.status = "done";
    computation.progress = 100;
    computation.message = "计算完成，可在下方查看结果并导出报告";
  } catch (err) {
    if (computation.cancelRequested) {
      computation.status = "idle";
      computation.message = "计算已取消";
      computation.cancelRequested = false;
    } else {
      computation.status = "error";
      computation.message = typeof err === "string" ? err : `计算失败：${String(err)}`;
    }
  } finally {
    unlisten?.();
  }
}
