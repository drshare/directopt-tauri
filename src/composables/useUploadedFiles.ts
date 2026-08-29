/**
 * 上传文件共享状态（FR-1 / FR-2）
 * UploadSection 解析上传文件后写入；computeTypes.buildCurveData 计算时读取曲线数据；
 * resultExport 导出时读取曲线文件原始字节（结果需填入上传的曲线文件内）。
 */
import { reactive } from "vue";
import {
  parseCurveWorkbook,
  parseInputWorkbook,
} from "@/lib/fileParsers";
import { params } from "./useParams";

export interface UploadedInputState {
  fileName: string;
  /** 成功回填的参数数量 */
  appliedCount: number;
  /** 无法识别的项目数量 */
  skippedCount: number;
  sheetName: string;
}

export interface UploadedCurveState {
  fileName: string;
  /** 原始文件字节（导出时基于该副本填写计算结果） */
  bytes: ArrayBuffer;
  /** 解析出的全年 8760h 曲线（与后端 CurveData 口径一致） */
  curve: import("@/lib/computeTypes").CurveDataOut;
  warnings: string[];
  sheetName: string;
}

export const uploadedFiles = reactive({
  input: null as UploadedInputState | null,
  curve: null as UploadedCurveState | null,
  inputError: "",
  curveError: "",
});

/** 解析输入文件并回填参数表单（界面修改仍可覆盖上传值） */
export function applyInputFile(fileName: string, bytes: ArrayBuffer): void {
  uploadedFiles.inputError = "";
  try {
    const result = parseInputWorkbook(bytes);
    for (const [key, value] of Object.entries(result.values)) {
      if (key in params) {
        (params as Record<string, unknown>)[key] = String(value);
      }
    }
    uploadedFiles.input = {
      fileName,
      appliedCount: result.appliedLabels.length,
      skippedCount: result.skippedLabels.length,
      sheetName: result.sheetName,
    };
  } catch (err) {
    uploadedFiles.input = null;
    uploadedFiles.inputError =
      err instanceof Error ? err.message : String(err);
    throw err;
  }
}

/** 解析曲线文件并保存 8760h 数据与原始字节 */
export function applyCurveFile(fileName: string, bytes: ArrayBuffer): void {
  uploadedFiles.curveError = "";
  try {
    const result = parseCurveWorkbook(bytes);
    uploadedFiles.curve = {
      fileName,
      bytes,
      curve: result.curve,
      warnings: result.warnings,
      sheetName: result.sheetName,
    };
  } catch (err) {
    uploadedFiles.curve = null;
    uploadedFiles.curveError =
      err instanceof Error ? err.message : String(err);
    throw err;
  }
}

/** 计算用曲线数据：优先使用上传文件，否则回退内置算例 */
export function activeCurve(): import("@/lib/computeTypes").CurveDataOut | null {
  return uploadedFiles.curve?.curve ?? null;
}

/** 导出用：上传曲线文件原始字节（无上传文件时为 null） */
export function uploadedCurveBytes(): ArrayBuffer | null {
  return uploadedFiles.curve?.bytes ?? null;
}
