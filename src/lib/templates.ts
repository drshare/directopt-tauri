import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import * as XLSX from "xlsx";

/** 模板元信息 */
export interface TemplateMeta {
  /** 文件名（与 public/templates 下一致） */
  name: string;
  label: string;
  description: string;
}

export const INPUT_TEMPLATE: TemplateMeta = {
  name: "inputtemplate_ldzl_3.0.xlsx",
  label: "输入文件模板",
  description: "技术参数、经济评价参数等静态数据",
};

export const CURVE_TEMPLATE: TemplateMeta = {
  name: "curvetemplate_ldzl_3.0.xlsx",
  label: "曲线及分时电价模板",
  description: "全年 8760h 负荷、风光标幺值、分时电价及过网费时序数据",
};

/** 预览时最多展示的行数（曲线模板有 8000+ 行，仅预览头部即可） */
const PREVIEW_MAX_ROWS = 100;

const MIME_XLSX =
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

export function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function templateUrl(fileName: string): string {
  return `${import.meta.env.BASE_URL}templates/${fileName}`;
}

async function fetchTemplateBytes(fileName: string): Promise<ArrayBuffer> {
  const res = await fetch(templateUrl(fileName));
  if (!res.ok) {
    throw new Error(`模板文件获取失败（HTTP ${res.status}）：${fileName}`);
  }
  return res.arrayBuffer();
}

export function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  const chunkSize = 0x8000;
  let binary = "";
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

/**
 * 下载模板文件。
 * - 浏览器环境：触发浏览器下载
 * - Tauri 桌面环境：通过 Rust 命令保存到系统下载目录，返回保存路径
 */
export async function downloadTemplate(meta: TemplateMeta): Promise<string> {
  const bytes = await fetchTemplateBytes(meta.name);

  if (isTauri()) {
    const data = arrayBufferToBase64(bytes);
    return await invoke<string>("save_template_file", {
      name: meta.name,
      data,
    });
  }

  const blob = new Blob([bytes], { type: MIME_XLSX });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = meta.name;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
  return meta.name;
}

/**
 * 在系统文件管理器中显示已保存的文件（仅 Tauri 环境有效）。
 * 返回是否成功触发显示。
 */
export async function revealSavedFile(path: string): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    await revealItemInDir(path);
    return true;
  } catch {
    // 部分环境未授予 reveal 权限时静默忽略
    return false;
  }
}

/** 单个 Sheet 的预览数据 */
export interface SheetPreview {
  name: string;
  rows: string[][];
  totalRows: number;
  truncated: boolean;
}

/** 工作簿预览数据 */
export interface WorkbookPreview {
  fileName: string;
  sheets: SheetPreview[];
}

/** 解析模板文件，返回各 Sheet 的表格预览 */
export async function loadTemplatePreview(
  meta: TemplateMeta,
): Promise<WorkbookPreview> {
  const bytes = await fetchTemplateBytes(meta.name);
  const workbook = XLSX.read(bytes, { type: "array" });

  const sheets: SheetPreview[] = workbook.SheetNames.map((sheetName) => {
    const worksheet = workbook.Sheets[sheetName];
    const allRows = XLSX.utils.sheet_to_json<string[]>(worksheet, {
      header: 1,
      defval: "",
      raw: false,
      blankrows: false,
    });
    const rows = allRows
      .slice(0, PREVIEW_MAX_ROWS)
      .map((row) => row.map((cell) => String(cell ?? "")));
    return {
      name: sheetName,
      rows,
      totalRows: allRows.length,
      truncated: allRows.length > rows.length,
    };
  });

  return { fileName: meta.name, sheets };
}
