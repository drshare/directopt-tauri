/**
 * 执行日志中心（全程可追溯）
 *
 * 记录一次完整计算执行的每一步：
 * 1. 上传 Excel 解析（输入参数 / 曲线数据）
 * 2. 开始计算传参（后端 ComputeParams + 曲线数据摘要）
 * 3. 计算过程（排队、遗传算法逐代进度、逐时结果、敏感性分析…）
 * 4. 计算结束（最优配置 + 指标结果摘要 / 失败原因）
 * 5. 结果导出
 *
 * 日志随历史记录一起保存（useHistory.HistoryRecord.logs），界面实时展示。
 */

import { reactive, readonly } from "vue";

export type LogLevel = "info" | "success" | "warn" | "error";

export interface LogEntry {
  /** 序号（会话内自增） */
  seq: number;
  /** 可读时间 HH:mm:ss */
  time: string;
  /** 毫秒时间戳（排序用） */
  ts: number;
  /** 阶段：文件解析 / 开始计算 / 计算进度 / 计算完成 / 计算失败 / 结果导出 */
  stage: string;
  level: LogLevel;
  message: string;
  /** 详细数据（JSON 或多行文本），界面可展开查看 */
  detail?: string;
}

const MAX_LOGS = 2000;

let seqCounter = 0;

function makeEntry(
  stage: string,
  level: LogLevel,
  message: string,
  detail?: string,
): LogEntry {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return {
    seq: ++seqCounter,
    time: `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`,
    ts: now.getTime(),
    stage,
    level,
    message,
    detail,
  };
}

const state = reactive({
  logs: [] as LogEntry[],
  /** 当前计算运行（开始计算 → 结束），供历史记录快照 */
  runOpen: false,
  runStartedAt: 0,
  runEntries: [] as LogEntry[],
});

export const executionLogs = readonly(state);

/** 追加一条日志（同时计入当前打开的计算运行） */
export function addLog(
  stage: string,
  level: LogLevel,
  message: string,
  detail?: string,
): void {
  const entry = makeEntry(stage, level, message, detail);
  state.logs.push(entry);
  if (state.logs.length > MAX_LOGS) state.logs.splice(0, state.logs.length - MAX_LOGS);
  if (state.runOpen) state.runEntries.push(entry);
}

/** 对象 → 缩进 JSON 文本（日志详情用） */
export function jsonDetail(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

/** 开始一次计算运行日志（开始计算时调用） */
export function beginRun(): void {
  state.runOpen = true;
  state.runStartedAt = Date.now();
  state.runEntries = [];
}

/**
 * 结束当前计算运行，返回本次运行的全部日志（用于随历史记录保存）。
 * 尚未开始运行时返回空数组。
 */
export function endRun(): LogEntry[] {
  state.runOpen = false;
  const entries = state.runEntries.slice();
  state.runEntries = [];
  return entries;
}

/** 清空界面日志（不影响已保存的历史记录） */
export function clearLogs(): void {
  state.logs = [];
}
