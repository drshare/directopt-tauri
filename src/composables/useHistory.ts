import { reactive, readonly } from "vue";
import type { ResultBundle } from "@/composables/useComputation";

/**
 * 计算历史（FR-10 的本地实现）
 * 每次计算完成后记录一份快照（参数 + 结果摘要），可回看、载入、删除。
 * 桌面端无后端时使用 localStorage 持久化；接入平台接口后可替换为服务端存储。
 */

export interface HistoryRecord {
  id: string;
  /** ISO 时间戳（排序用） */
  at: string;
  /** 中文可读时间（展示用） */
  atLabel: string;
  /** 发电及负荷曲线文件名 */
  curveFile: string;
  schemeLabel: string;
  objectiveLabel: string;
  /** 计算时的参数快照（字符串参数 + 时段数组） */
  params: Record<string, string | number | number[]>;
  /** 结果快照 */
  result: ResultBundle;
}

const STORAGE_KEY = "directopt.computationHistory";
const MAX_RECORDS = 50;

function readStorage(): HistoryRecord[] {
  try {
    if (typeof localStorage === "undefined") return [];
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as HistoryRecord[]) : [];
  } catch {
    return [];
  }
}

function writeStorage(records: HistoryRecord[]): void {
  try {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(records));
  } catch {
    /* 存储不可用（隐私模式/配额）时静默降级为内存历史 */
  }
}

function newId(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function toLabel(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ` +
    `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
  );
}

export const history = reactive<HistoryRecord[]>(readStorage());
export const historyState = readonly(history);

export interface HistoryInput {
  curveFile: string;
  schemeLabel: string;
  objectiveLabel: string;
  params: Record<string, string | number | number[]>;
  result: ResultBundle;
}

/** 新增一条历史记录（最新在前，超出上限丢弃最旧） */
export function addHistory(input: HistoryInput): HistoryRecord {
  const now = new Date();
  const record: HistoryRecord = {
    id: newId(),
    at: now.toISOString(),
    atLabel: toLabel(now),
    ...input,
  };
  history.unshift(record);
  if (history.length > MAX_RECORDS) history.length = MAX_RECORDS;
  writeStorage([...history]);
  return record;
}

/** 删除单条历史记录 */
export function removeHistory(id: string): void {
  const index = history.findIndex((r) => r.id === id);
  if (index === -1) return;
  history.splice(index, 1);
  writeStorage([...history]);
}

/** 清空全部历史记录 */
export function clearHistory(): void {
  history.length = 0;
  writeStorage([]);
}
