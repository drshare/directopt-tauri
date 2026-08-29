/**
 * 数据文件仓库（文件管理 FR-10）
 * 上传的输入文件、曲线文件与导出的结果文件在本地归档，
 * 支持查看列表、重新载入、在系统文件管理器中定位与删除。
 * Tauri 桌面端：文件落盘到 <app_data_dir>/files/，跨会话持久化；
 * 浏览器环境：保存在内存 Map 中（仅当前会话有效，供开发演示）。
 */
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  arrayBufferToBase64,
  isTauri,
  revealSavedFile,
} from "@/lib/templates";
import { addLog } from "./useExecutionLog";

/** 文件类别 */
export type FileKind = "input" | "curve" | "result";

/** 归档文件元信息（与后端 StoredFileMeta 对齐） */
export interface StoredFileMeta {
  id: string;
  name: string;
  kind: FileKind;
  size: number;
  /** 归档时间（Unix 毫秒） */
  savedAtMs: number;
  /** 磁盘绝对路径（仅 Tauri 环境有效） */
  path: string;
}

export const FILE_KIND_LABELS: Record<FileKind, string> = {
  input: "输入文件",
  curve: "曲线文件",
  result: "结果文件",
};

/** 归档文件仓库状态 */
export const fileStore = reactive({
  files: [] as StoredFileMeta[],
  loaded: false,
  loading: false,
});

/** 浏览器环境的内存仓库（仅当前会话有效） */
const memoryFiles = new Map<string, { meta: StoredFileMeta; bytes: ArrayBuffer }>();
let memorySeq = 0;

/** base64 → ArrayBuffer（读取后端归档文件用） */
function base64ToArrayBuffer(data: string): ArrayBuffer {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
}

/** 刷新文件列表 */
export async function refreshFiles(): Promise<void> {
  if (!isTauri()) {
    fileStore.files = [...memoryFiles.values()]
      .map((v) => ({ ...v.meta }))
      .sort((a, b) => b.savedAtMs - a.savedAtMs || a.id.localeCompare(b.id));
    fileStore.loaded = true;
    return;
  }
  fileStore.loading = true;
  try {
    fileStore.files = await invoke<StoredFileMeta[]>("list_data_files");
    fileStore.loaded = true;
  } catch (err) {
    addLog("文件管理", "error", `读取文件列表失败：${String(err)}`);
  } finally {
    fileStore.loading = false;
  }
}

/** 归档文件，成功返回元信息，失败返回 null（不中断调用方主流程） */
export async function saveManagedFile(
  name: string,
  bytes: ArrayBuffer,
  kind: FileKind,
): Promise<StoredFileMeta | null> {
  try {
    if (isTauri()) {
      const meta = await invoke<StoredFileMeta>("save_data_file", {
        name,
        data: arrayBufferToBase64(bytes),
        kind,
      });
      fileStore.files = [meta, ...fileStore.files];
      return meta;
    }
    // 浏览器环境：内存归档
    const id = `mem/${++memorySeq}_${name}`;
    const meta: StoredFileMeta = {
      id,
      name,
      kind,
      size: bytes.byteLength,
      savedAtMs: Date.now(),
      path: "",
    };
    memoryFiles.set(id, { meta, bytes: bytes.slice(0) });
    fileStore.files = [meta, ...fileStore.files];
    return meta;
  } catch (err) {
    addLog("文件管理", "error", `归档「${name}」失败：${String(err)}`);
    return null;
  }
}

/** 读取归档文件内容（重新载入曲线/输入文件用） */
export async function readManagedFile(id: string): Promise<ArrayBuffer> {
  if (!isTauri()) {
    const hit = memoryFiles.get(id);
    if (!hit) throw new Error("文件不存在（内存归档仅当前会话有效）");
    return hit.bytes.slice(0);
  }
  const data = await invoke<string>("read_data_file", { id });
  return base64ToArrayBuffer(data);
}

/** 删除单个归档文件 */
export async function deleteManagedFile(id: string): Promise<void> {
  if (isTauri()) {
    await invoke("delete_data_file", { id });
  } else {
    memoryFiles.delete(id);
  }
  fileStore.files = fileStore.files.filter((f) => f.id !== id);
  addLog("文件管理", "info", `已删除归档文件「${id}」`);
}

/** 清空指定类别（缺省为全部类别）的归档文件 */
export async function clearManagedFiles(kind?: FileKind): Promise<void> {
  if (isTauri()) {
    await invoke("clear_data_files", { kind: kind ?? null });
  } else if (kind) {
    for (const id of [...memoryFiles.keys()]) {
      if (memoryFiles.get(id)?.meta.kind === kind) memoryFiles.delete(id);
    }
  } else {
    memoryFiles.clear();
  }
  fileStore.files = kind
    ? fileStore.files.filter((f) => f.kind !== kind)
    : [];
  addLog(
    "文件管理",
    "info",
    kind ? `已清空全部「${FILE_KIND_LABELS[kind]}」` : "已清空全部归档文件",
  );
}

/** 在系统文件管理器中定位归档文件（仅 Tauri 环境有效） */
export async function locateManagedFile(meta: StoredFileMeta): Promise<boolean> {
  if (!meta.path) return false;
  const ok = await revealSavedFile(meta.path);
  if (!ok) addLog("文件管理", "warn", "定位失败：当前环境未授予文件定位权限");
  return ok;
}

/** 字节数展示 */
export function formatSize(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / 1024 / 1024).toFixed(2)} MB`;
}
