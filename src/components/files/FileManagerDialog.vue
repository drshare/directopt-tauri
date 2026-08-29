<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  ExternalLink,
  FileSpreadsheet,
  FolderOpen,
  Loader2,
  Trash2,
  Upload,
} from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { applyCurveFile, applyInputFile } from "@/composables/useUploadedFiles";
import {
  clearManagedFiles,
  deleteManagedFile,
  FILE_KIND_LABELS,
  fileStore,
  formatSize,
  locateManagedFile,
  readManagedFile,
  refreshFiles,
  type FileKind,
  type StoredFileMeta,
} from "@/composables/useFileStore";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: "update:open", value: boolean): void }>();

const KINDS: FileKind[] = ["input", "curve", "result"];

const busyId = ref<string | null>(null);

onMounted(() => {
  if (!fileStore.loaded) void refreshFiles();
});

const byKind = (kind: FileKind) =>
  computed(() => fileStore.files.filter((f) => f.kind === kind));

const inputFiles = byKind("input");
const curveFiles = byKind("curve");
const resultFiles = byKind("result");

function timeLabel(ms: number): string {
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** 在系统文件管理器中定位（仅桌面端有效） */
async function locate(meta: StoredFileMeta) {
  await locateManagedFile(meta);
}

/** 重新载入输入文件：重新解析并回填参数 */
async function loadInput(meta: StoredFileMeta) {
  busyId.value = meta.id;
  try {
    const bytes = await readManagedFile(meta.id);
    applyInputFile(meta.name, bytes);
  } catch (err) {
    // 解析失败信息已写入执行日志
    void err;
  } finally {
    busyId.value = null;
  }
}

/** 重新载入曲线文件：重新解析并作为计算数据源 */
async function loadCurve(meta: StoredFileMeta) {
  busyId.value = meta.id;
  try {
    const bytes = await readManagedFile(meta.id);
    applyCurveFile(meta.name, bytes);
  } catch (err) {
    void err;
  } finally {
    busyId.value = null;
  }
}

function removeFile(meta: StoredFileMeta) {
  if (window.confirm(`确定删除归档文件「${meta.name}」吗？`)) {
    void deleteManagedFile(meta.id);
  }
}

function clearKind(kind: FileKind) {
  const count = byKind(kind).value.length;
  if (count === 0) return;
  if (window.confirm(`确定清空全部 ${count} 个「${FILE_KIND_LABELS[kind]}」吗？`)) {
    void clearManagedFiles(kind);
  }
}
</script>

<template>
  <Dialog :open="props.open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2">
          <FolderOpen class="size-5 text-sky-600" />
          文件管理
        </DialogTitle>
        <DialogDescription>
          归档的输入文件、曲线文件与结果文件，可重新载入、定位或删除
        </DialogDescription>
      </DialogHeader>

      <Tabs default-value="curve" class="gap-3">
        <TabsList class="w-full justify-start">
          <TabsTrigger value="curve" class="gap-1.5">
            曲线文件
            <Badge variant="secondary" class="px-1.5">{{ curveFiles.length }}</Badge>
          </TabsTrigger>
          <TabsTrigger value="input" class="gap-1.5">
            输入文件
            <Badge variant="secondary" class="px-1.5">{{ inputFiles.length }}</Badge>
          </TabsTrigger>
          <TabsTrigger value="result" class="gap-1.5">
            结果文件
            <Badge variant="secondary" class="px-1.5">{{ resultFiles.length }}</Badge>
          </TabsTrigger>
        </TabsList>

        <TabsContent
          v-for="kind in KINDS"
          :key="kind"
          :value="kind"
          class="min-h-64"
        >
          <!-- 空态 -->
          <div
            v-if="byKind(kind).value.length === 0"
            class="flex items-center justify-center gap-2 rounded-lg border border-dashed py-10 text-sm text-muted-foreground"
          >
            <FileSpreadsheet class="size-4" />
            <template v-if="kind === 'input'">暂无归档输入文件，上传输入文件后自动归档</template>
            <template v-else-if="kind === 'curve'">暂无归档曲线文件，上传曲线文件后自动归档</template>
            <template v-else>暂无归档结果文件，导出计算结果后自动归档</template>
          </div>

          <!-- 文件列表 -->
          <ul v-else class="max-h-80 space-y-2 overflow-y-auto pr-1">
            <li
              v-for="file in byKind(kind).value"
              :key="file.id"
              class="flex items-center gap-3 rounded-lg border bg-muted/20 p-3"
            >
              <FileSpreadsheet class="size-5 shrink-0 text-emerald-600" aria-hidden="true" />
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">{{ file.name }}</p>
                <p class="text-xs text-muted-foreground">
                  {{ formatSize(file.size) }} · 归档于 {{ timeLabel(file.savedAtMs) }}
                </p>
              </div>
              <div class="flex shrink-0 items-center gap-1">
                <Button
                  v-if="kind === 'input'"
                  variant="outline"
                  size="sm"
                  class="gap-1.5"
                  :disabled="busyId === file.id"
                  @click="loadInput(file)"
                >
                  <Loader2 v-if="busyId === file.id" class="size-3.5 animate-spin" />
                  <Upload v-else class="size-3.5" />
                  载入
                </Button>
                <Button
                  v-else-if="kind === 'curve'"
                  variant="outline"
                  size="sm"
                  class="gap-1.5"
                  :disabled="busyId === file.id"
                  @click="loadCurve(file)"
                >
                  <Loader2 v-if="busyId === file.id" class="size-3.5 animate-spin" />
                  <Upload v-else class="size-3.5" />
                  载入
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  title="在系统文件管理器中定位"
                  @click="locate(file)"
                >
                  <ExternalLink class="size-3.5 text-muted-foreground" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  title="删除该文件"
                  @click="removeFile(file)"
                >
                  <Trash2 class="size-3.5 text-muted-foreground" />
                </Button>
              </div>
            </li>
          </ul>

          <!-- 底部：清空该类别 -->
          <div
            v-if="byKind(kind).value.length > 0"
            class="mt-3 flex items-center justify-between text-xs text-muted-foreground"
          >
            <span>共 {{ byKind(kind).value.length }} 个文件</span>
            <Button
              variant="ghost"
              size="sm"
              class="gap-1.5 text-muted-foreground"
              @click="clearKind(kind)"
            >
              <Trash2 class="size-3.5" />
              清空
            </Button>
          </div>
        </TabsContent>
      </Tabs>
    </DialogContent>
  </Dialog>
</template>
