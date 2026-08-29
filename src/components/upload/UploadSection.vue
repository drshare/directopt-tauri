<script setup lang="ts">
import { ref } from "vue";
import {
  Building2,
  CloudUpload,
  Download,
  Eye,
  FileSpreadsheet,
  Grid3x3,
  Sun,
  Wind,
} from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { params } from "@/composables/useParams";
import {
  applyCurveFile,
  applyInputFile,
  uploadedFiles,
} from "@/composables/useUploadedFiles";
import {
  CURVE_TEMPLATE,
  INPUT_TEMPLATE,
  downloadTemplate,
  revealSavedFile,
  type TemplateMeta,
} from "@/lib/templates";
import TemplatePreviewDialog from "./TemplatePreviewDialog.vue";

const inputUploadTime = ref("");
const curveUploadTime = ref("");

const fileInputRef = ref<HTMLInputElement | null>(null);
const curveInputRef = ref<HTMLInputElement | null>(null);
const inputDragging = ref(false);
const curveDragging = ref(false);

const previewOpen = ref(false);
const previewTemplate = ref<TemplateMeta | null>(null);

/** 系统示意图：风电 / 光伏 / 储能 / 数据中心(负荷) / 电网 */
const systemNodes = [
  { label: "风电", icon: Wind, color: "text-sky-500" },
  { label: "光伏", icon: Sun, color: "text-amber-500" },
  { label: "储能", icon: Grid3x3, color: "text-emerald-500" },
  { label: "数据中心", icon: Building2, color: "text-violet-500" },
  { label: "电网", icon: Grid3x3, color: "text-rose-500" },
];

/** 读取文件字节并真实解析（参数回填 / 8760h 曲线），失败时提示错误 */
async function setFile(file: File, kind: "input" | "curve") {
  const time = new Date().toLocaleString("zh-CN");
  try {
    const bytes = await file.arrayBuffer();
    if (kind === "input") {
      applyInputFile(file.name, bytes);
      params.inputFile = file.name;
      inputUploadTime.value = time;
    } else {
      applyCurveFile(file.name, bytes);
      params.curveFile = file.name;
      curveUploadTime.value = time;
    }
  } catch (err) {
    window.alert(
      `${kind === "input" ? "输入文件" : "曲线文件"}解析失败：${
        err instanceof Error ? err.message : String(err)
      }`,
    );
  }
}

function onFileChange(event: Event, kind: "input" | "curve") {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  if (!file) return;
  setFile(file, kind);
  target.value = "";
}

function onDrop(event: DragEvent, kind: "input" | "curve") {
  if (kind === "input") {
    inputDragging.value = false;
  } else {
    curveDragging.value = false;
  }
  const file = event.dataTransfer?.files?.[0];
  if (!file) return;
  setFile(file, kind);
}

function openPreview(template: TemplateMeta) {
  previewTemplate.value = template;
  previewOpen.value = true;
}

async function onDownloadTemplate(template: TemplateMeta) {
  try {
    const savedPath = await downloadTemplate(template);
    const revealed = await revealSavedFile(savedPath);
    if (!revealed) {
      window.alert(
        savedPath === template.name
          ? `模板 ${template.name} 已开始下载`
          : `模板已保存到：${savedPath}`,
      );
    }
  } catch (e) {
    window.alert(`模板下载失败：${e instanceof Error ? e.message : String(e)}`);
  }
}
</script>

<template>
  <div class="space-y-4">
    <!-- 系统示意图 -->
    <div class="flex flex-wrap items-center justify-center gap-x-2 gap-y-3 rounded-lg border bg-muted/30 p-3 sm:gap-3 sm:p-4">
        <template v-for="(node, i) in systemNodes" :key="node.label">
          <div class="flex flex-col items-center gap-1 rounded-md border bg-background px-3 py-2 shadow-sm sm:px-4">
            <component :is="node.icon" class="size-6" :class="node.color" />
            <span class="text-xs font-medium">{{ node.label }}</span>
          </div>
          <span v-if="i < systemNodes.length - 1" class="text-muted-foreground">→</span>
        </template>
      </div>

      <!-- 文件上传 -->
      <div class="grid gap-4 lg:grid-cols-2">
        <div class="rounded-lg border p-4">
          <div class="mb-1 flex items-center gap-2">
            <Label class="text-sm font-medium">上传输入文件</Label>
            <Badge variant="secondary">可选</Badge>
          </div>
          <p class="mb-3 text-xs text-muted-foreground">
            可上传技术参数、经济评价参数等静态数据；也可直接在下方表单中填写
          </p>
          <div
            class="mb-3 flex items-center gap-1 rounded-md border border-emerald-200 bg-emerald-50/60 p-1"
          >
            <Button
              variant="ghost"
              size="sm"
              class="min-w-0 flex-1 justify-start text-xs text-emerald-700"
              :title="INPUT_TEMPLATE.description"
              @click="onDownloadTemplate(INPUT_TEMPLATE)"
            >
              <Download class="size-4 shrink-0 text-emerald-600" />
              <span class="truncate">下载模板：{{ INPUT_TEMPLATE.name }}</span>
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title="预览模板"
              @click="openPreview(INPUT_TEMPLATE)"
            >
              <Eye class="size-4 text-muted-foreground" />
            </Button>
          </div>
          <button
            type="button"
            class="flex w-full flex-col items-center justify-center gap-1.5 rounded-lg border-2 border-dashed px-4 py-6 transition-colors hover:border-emerald-400 hover:bg-emerald-50/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-offset-2 dark:hover:bg-emerald-950/20"
            :class="
              inputDragging
                ? 'border-emerald-500 bg-emerald-50/70'
                : 'border-muted-foreground/30'
            "
            aria-label="上传输入文件：点击选择或拖拽 .xlsx 文件到此处"
            @click="fileInputRef?.click()"
            @dragover.prevent="inputDragging = true"
            @dragleave.prevent="inputDragging = false"
            @drop.prevent="onDrop($event, 'input')"
          >
            <CloudUpload
              class="size-8 text-emerald-500 transition-transform"
              :class="inputDragging && 'scale-110'"
              aria-hidden="true"
            />
            <span class="text-sm">
              拖拽文件到此处，或<span class="font-medium text-emerald-600 underline underline-offset-2">点击选择</span>
            </span>
            <span class="text-xs text-muted-foreground">仅支持 .xlsx 格式</span>
          </button>
          <input
            ref="fileInputRef"
            type="file"
            accept=".xlsx"
            class="sr-only"
            tabindex="-1"
            aria-hidden="true"
            @change="onFileChange($event, 'input')"
          />
          <p class="mt-2 text-xs" aria-live="polite">
            <template v-if="params.inputFile">
              <Badge variant="outline" class="gap-1 border-emerald-600/40 bg-emerald-50 text-emerald-700">
                <FileSpreadsheet class="size-3" />
                <span class="max-w-48 truncate align-middle">{{ params.inputFile }}</span>
              </Badge>
              <span class="ml-2 text-muted-foreground">{{ inputUploadTime }}</span>
            </template>
            <template v-else>
              <span class="text-muted-foreground">未选择文件</span>
            </template>
          </p>
          <p v-if="uploadedFiles.input" class="mt-1 text-xs text-emerald-700">
            已解析「{{ uploadedFiles.input.sheetName }}」并回填 {{ uploadedFiles.input.appliedCount }} 项参数<template v-if="uploadedFiles.input.skippedCount > 0">（{{ uploadedFiles.input.skippedCount }} 项未识别，保留界面当前值）</template>
          </p>
          <p v-else-if="uploadedFiles.inputError" class="mt-1 text-xs text-destructive">
            {{ uploadedFiles.inputError }}
          </p>
        </div>

        <div class="rounded-lg border border-amber-300/60 p-4">
          <div class="mb-1 flex items-center gap-2">
            <Label class="text-sm font-medium">上传发电及负荷曲线</Label>
            <Badge class="bg-amber-500 text-white">必选</Badge>
          </div>
          <p class="mb-3 text-xs text-muted-foreground">
            必填全年 8760h 负荷、风光标幺值、分时电价及过网费四项费用时序数据
          </p>
          <div
            class="mb-3 flex items-center gap-1 rounded-md border border-amber-200 bg-amber-50/60 p-1"
          >
            <Button
              variant="ghost"
              size="sm"
              class="min-w-0 flex-1 justify-start text-xs text-amber-700"
              :title="CURVE_TEMPLATE.description"
              @click="onDownloadTemplate(CURVE_TEMPLATE)"
            >
              <Download class="size-4 shrink-0 text-amber-600" />
              <span class="truncate">下载模板：{{ CURVE_TEMPLATE.name }}</span>
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title="预览模板"
              @click="openPreview(CURVE_TEMPLATE)"
            >
              <Eye class="size-4 text-muted-foreground" />
            </Button>
          </div>
          <button
            type="button"
            class="flex w-full flex-col items-center justify-center gap-1.5 rounded-lg border-2 border-dashed px-4 py-6 transition-colors hover:border-amber-400 hover:bg-amber-50/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber-500 focus-visible:ring-offset-2 dark:hover:bg-amber-950/20"
            :class="
              curveDragging
                ? 'border-amber-500 bg-amber-50/70'
                : 'border-muted-foreground/30'
            "
            aria-label="上传发电及负荷曲线：点击选择或拖拽 .xlsx 文件到此处"
            @click="curveInputRef?.click()"
            @dragover.prevent="curveDragging = true"
            @dragleave.prevent="curveDragging = false"
            @drop.prevent="onDrop($event, 'curve')"
          >
            <CloudUpload
              class="size-8 text-amber-500 transition-transform"
              :class="curveDragging && 'scale-110'"
              aria-hidden="true"
            />
            <span class="text-sm">
              拖拽文件到此处，或<span class="font-medium text-amber-600 underline underline-offset-2">点击选择</span>
            </span>
            <span class="text-xs text-muted-foreground">仅支持 .xlsx 格式</span>
          </button>
          <input
            ref="curveInputRef"
            type="file"
            accept=".xlsx"
            class="sr-only"
            tabindex="-1"
            aria-hidden="true"
            @change="onFileChange($event, 'curve')"
          />
          <p class="mt-2 text-xs" aria-live="polite">
            <template v-if="params.curveFile">
              <Badge variant="outline" class="gap-1 border-amber-600/40 bg-amber-50 text-amber-700">
                <FileSpreadsheet class="size-3" />
                <span class="max-w-48 truncate align-middle">{{ params.curveFile }}</span>
              </Badge>
              <span class="ml-2 text-muted-foreground">{{ curveUploadTime }}</span>
            </template>
            <template v-else>
              <span class="text-muted-foreground">未选择文件</span>
            </template>
          </p>
          <p v-if="uploadedFiles.curve" class="mt-1 text-xs text-amber-700">
            已解析「{{ uploadedFiles.curve.sheetName }}」：全年 8760h × 8 列时序数据，计算将使用该文件数据
            <template v-if="uploadedFiles.curve.warnings.length > 0">
              <br /><span class="text-amber-600">{{ uploadedFiles.curve.warnings.join("；") }}</span>
            </template>
          </p>
          <p v-else-if="uploadedFiles.curveError" class="mt-1 text-xs text-destructive">
            {{ uploadedFiles.curveError }}
          </p>
        </div>
      </div>
    </div>

    <!-- 模板预览弹窗 -->
    <TemplatePreviewDialog v-model:open="previewOpen" :template="previewTemplate" />
</template>