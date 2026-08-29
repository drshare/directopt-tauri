<script setup lang="ts">
import { computed } from "vue";
import { ChevronRight } from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import type { LogEntry, LogLevel } from "@/composables/useExecutionLog";

const props = defineProps<{
  entries: LogEntry[];
  /** 默认展开详情（历史记录回看时使用） */
  defaultOpen?: boolean;
}>();

const ordered = computed(() => [...props.entries].reverse());

const levelStyle: Record<LogLevel, { dot: string; label: string }> = {
  info: { dot: "bg-sky-500", label: "信息" },
  success: { dot: "bg-emerald-500", label: "成功" },
  warn: { dot: "bg-amber-500", label: "警告" },
  error: { dot: "bg-destructive", label: "错误" },
};

const stageStyle: Record<string, string> = {
  文件解析: "bg-emerald-50 text-emerald-700 border-emerald-200",
  开始计算: "bg-violet-50 text-violet-700 border-violet-200",
  计算进度: "bg-sky-50 text-sky-700 border-sky-200",
  计算完成: "bg-emerald-50 text-emerald-700 border-emerald-200",
  计算失败: "bg-red-50 text-red-700 border-red-200",
  结果导出: "bg-amber-50 text-amber-700 border-amber-200",
};

function stageClass(stage: string): string {
  return stageStyle[stage] ?? "bg-muted text-muted-foreground border-border";
}
</script>

<template>
  <ol class="max-h-96 space-y-1 overflow-y-auto rounded-lg border bg-muted/20 p-2 font-mono text-xs">
    <li v-for="entry in ordered" :key="entry.seq">
      <details :open="defaultOpen && entry.detail !== undefined">
        <summary class="flex cursor-pointer list-none items-start gap-2 rounded px-1.5 py-1 hover:bg-muted/60">
          <ChevronRight
            v-if="entry.detail"
            class="rotate-on-open mt-0.5 size-3 shrink-0 text-muted-foreground transition-transform"
          />
          <span v-else class="size-3 shrink-0" />
          <span class="shrink-0 text-muted-foreground/70">{{ entry.time }}</span>
          <span
            class="mt-1 size-1.5 shrink-0 rounded-full"
            :class="levelStyle[entry.level].dot"
            :title="levelStyle[entry.level].label"
          />
          <Badge
            variant="outline"
            class="h-4 shrink-0 px-1 text-[10px]"
            :class="stageClass(entry.stage)"
          >
            {{ entry.stage }}
          </Badge>
          <span class="min-w-0 break-all text-foreground/90">{{ entry.message }}</span>
        </summary>
        <pre
          v-if="entry.detail"
          class="mx-2 mt-1 max-h-64 overflow-auto whitespace-pre-wrap rounded border bg-background p-2 text-[11px] leading-relaxed text-muted-foreground"
        >{{ entry.detail }}</pre>
      </details>
    </li>
  </ol>
</template>

<style scoped>
summary::-webkit-details-marker {
  display: none;
}
details[open] > summary .rotate-on-open {
  transform: rotate(90deg);
}
</style>

