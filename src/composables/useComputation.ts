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
import { addLog, jsonDetail } from "@/composables/useExecutionLog";
import { uploadedFiles } from "@/composables/useUploadedFiles";

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
  addLog("计算进度", "warn", "当前为浏览器环境（非 Tauri 桌面端），使用内置算例演示流程");
  addLog("计算进度", "info", "排队中：任务等待执行");
  await delay(1500);

  computation.status = "running";
  computation.progress = 0;
  computation.message = "努力计算中…请勿关闭页面，否则无法显示计算结果";
  for (let progress = 0; progress <= 100; progress += 10) {
    addLog("计算进度", "info", `演示进度 ${progress}%`);
    computation.progress = progress;
    await delay(120);
  }

  computation.status = "done";
  computation.message = "计算完成，可在下方查看结果并导出报告";
  addLog("计算完成", "success", "演示流程完成（结果为内置算例数据）");
}

/**
 * 真实计算流程（全部计算在后端 Rust 执行，确保准确性）：
 * 1. 组装参数与曲线，invoke("start_compute") 提交后端任务；
 * 2. 监听 compute://progress 事件更新进度（遗传算法逐代回报）；
 * 3. 命令返回后用后端结果负载替换当前展示结果与逐时/敏感性数据；
 * 4. 浏览器（非 Tauri）环境回退为演示流程。
 * 状态机与消息口径保持不变；全部步骤写入执行日志。
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

  // ---- 开始计算传参日志：完整入参 + 曲线数据摘要 ----
  const paramsPayload = buildComputeParams();
  const curvesPayload = buildCurveData();
  const curveStat = (name: string, s: number[]) => ({
    列: name,
    长度: s.length,
    首值: Number(s[0]?.toFixed(6)),
    末值: Number(s[s.length - 1]?.toFixed(6)),
    均值: Number((s.reduce((a, b) => a + b, 0) / s.length).toFixed(6)),
  });
  addLog(
    "开始计算",
    "info",
    "提交计算任务至后端（遗传算法 + 8760h 仿真），传参详情见日志",
    jsonDetail({
      计算参数: paramsPayload,
      曲线数据: {
        用电负荷: curveStat("负荷", curvesPayload.load),
        风电标幺值: curveStat("风电", curvesPayload.windPu),
        光伏标幺值: curveStat("光伏", curvesPayload.pvPu),
        电量电价: curveStat("电价", curvesPayload.price),
        数据来源: uploadedFiles.curve ? `上传文件「${uploadedFiles.curve.fileName}」` : "内置算例",
      },
    }),
  );

  let unlisten: (() => void) | null = null;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<ProgressPayload>("compute://progress", (e) => {
      computation.status = "running";
      computation.progress = Math.max(0, Math.min(100, Math.round(e.payload.progress)));
      computation.message = e.payload.message;
      addLog(
        "计算进度",
        "info",
        `[${computation.progress}%] ${e.payload.message}`,
      );
    });

    const payload = await invoke<ComputeResultPayload>("start_compute", {
      params: paramsPayload,
      curves: curvesPayload,
    });

    // ---- 计算完成：结果摘要日志 ----
    applyComputeResult(payload);
    result.headline = payload.headline.map(formatMetricItem);
    result.invest = payload.invest.map(formatMetricItem);
    result.opex = payload.opex.map(formatMetricItem);

    computation.status = "done";
    computation.progress = 100;
    computation.message = "计算完成，可在下方查看结果并导出报告";
    const elapsed = ((Date.now() - computation.startedAt) / 1000).toFixed(1);
    addLog(
      "计算完成",
      "success",
      `计算完成，耗时 ${elapsed} 秒。最优配置：风电 ${payload.best.windKw / 1000} MW / 光伏 ${payload.best.pvKw / 1000} MW / 储能 ${payload.best.essKwh / 1000} MWh`,
      jsonDetail({
        耗时秒: Number(elapsed),
        最优配置: payload.best,
        适应度: payload.best.fitness,
        头部指标: payload.headline,
        投资构成: payload.invest,
        年运行成本构成: payload.opex,
        全年电量指标: payload.energyStats,
        敏感性分析组数: payload.sensitivity.length,
        逐时序列长度: payload.balance.load.length,
      }),
    );
  } catch (err) {
    if (computation.cancelRequested) {
      computation.status = "idle";
      computation.message = "计算已取消";
      computation.cancelRequested = false;
      addLog("计算进度", "warn", "计算已被用户取消");
    } else {
      computation.status = "error";
      computation.message = typeof err === "string" ? err : `计算失败：${String(err)}`;
      addLog("计算失败", "error", `计算失败：${computation.message}`);
    }
  } finally {
    unlisten?.();
  }
}
