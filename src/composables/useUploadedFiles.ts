/**
 * 上传文件共享状态（FR-1 / FR-2）
 * UploadSection 解析上传文件后写入；computeTypes.buildCurveData 计算时读取曲线数据；
 * resultExport 导出时读取曲线文件原始字节（结果需填入上传的曲线文件内）。
 * 全部解析步骤写入执行日志（useExecutionLog）。
 */
import { reactive } from "vue";
import {
  parseCurveWorkbook,
  parseInputWorkbook,
} from "@/lib/fileParsers";
import { addLog, jsonDetail } from "./useExecutionLog";
import { params } from "./useParams";
import { saveManagedFile } from "./useFileStore";

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

/** 曲线序列摘要（日志详情用） */
function curveSummary(curve: import("@/lib/computeTypes").CurveDataOut) {
  const stat = (name: string, s: number[]) => {
    const sum = s.reduce((a, b) => a + b, 0);
    return {
      列: name,
      长度: s.length,
      首值: Number(s[0]?.toFixed(6)),
      末值: Number(s[s.length - 1]?.toFixed(6)),
      均值: Number((sum / s.length).toFixed(6)),
      总和: Number(sum.toFixed(3)),
    };
  };
  return [
    stat("用电负荷", curve.load),
    stat("风电标幺值", curve.windPu),
    stat("光伏标幺值", curve.pvPu),
    stat("电量电价", curve.price),
    stat("线损费", curve.lossFee),
    stat("输配电价", curve.tduFee),
    stat("系统运行费", curve.systemFee),
    stat("基金附加", curve.fundFee),
  ];
}

/** 解析输入文件并回填参数表单（界面修改仍可覆盖上传值） */
export function applyInputFile(fileName: string, bytes: ArrayBuffer): void {
  uploadedFiles.inputError = "";
  addLog("文件解析", "info", `开始解析输入文件「${fileName}」（${(bytes.byteLength / 1024).toFixed(1)} KB）`);
  try {
    const result = parseInputWorkbook(bytes);
    const before = { ...params };
    for (const [key, value] of Object.entries(result.values)) {
      if (key in params) {
        (params as Record<string, unknown>)[key] = String(value);
      }
    }
    // 记录被回填的参数变化
    const changes = Object.entries(result.values)
      .filter(([key]) => key in params)
      .map(([key, value]) => ({
        参数: key,
        原值: (before as Record<string, unknown>)[key],
        新值: String(value),
      }));
    uploadedFiles.input = {
      fileName,
      appliedCount: result.appliedLabels.length,
      skippedCount: result.skippedLabels.length,
      sheetName: result.sheetName,
    };
    // 归档原始文件（文件管理 FR-10）
    void saveManagedFile(fileName, bytes, "input");
    addLog(
      "文件解析",
      "success",
      `输入文件解析成功：工作表「${result.sheetName}」，识别 ${result.appliedLabels.length} 项参数并回填` +
        (result.skippedLabels.length > 0
          ? `，${result.skippedLabels.length} 项未识别（${result.skippedLabels.join("、")}），保留界面当前值`
          : ""),
      jsonDetail({ 文件: fileName, 回填参数: changes }),
    );
  } catch (err) {
    uploadedFiles.input = null;
    uploadedFiles.inputError = err instanceof Error ? err.message : String(err);
    addLog("文件解析", "error", `输入文件解析失败：${uploadedFiles.inputError}`);
    throw err;
  }
}

/** 解析曲线文件并保存 8760h 数据与原始字节 */
export function applyCurveFile(fileName: string, bytes: ArrayBuffer): void {
  uploadedFiles.curveError = "";
  addLog("文件解析", "info", `开始解析发电及负荷曲线「${fileName}」（${(bytes.byteLength / 1024).toFixed(1)} KB）`);
  try {
    const result = parseCurveWorkbook(bytes);
    uploadedFiles.curve = {
      fileName,
      bytes,
      curve: result.curve,
      warnings: result.warnings,
      sheetName: result.sheetName,
    };
    // 归档原始文件（文件管理 FR-10）
    void saveManagedFile(fileName, bytes, "curve");
    addLog(
      "文件解析",
      "success",
      `曲线文件解析成功：工作表「${result.sheetName}」，全年 8760h × 8 列时序数据，计算将使用该文件数据` +
        (result.warnings.length > 0 ? `；警告：${result.warnings.join("；")}` : ""),
      jsonDetail({
        文件: fileName,
        曲线摘要: curveSummary(result.curve),
      }),
    );
  } catch (err) {
    uploadedFiles.curve = null;
    uploadedFiles.curveError = err instanceof Error ? err.message : String(err);
    addLog("文件解析", "error", `曲线文件解析失败：${uploadedFiles.curveError}`);
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
