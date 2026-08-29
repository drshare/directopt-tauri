<script setup lang="ts">
import { computed } from "vue";
import { ScrollText, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import LogList from "./LogList.vue";
import {
  clearLogs,
  executionLogs,
  type LogEntry,
} from "@/composables/useExecutionLog";

const entries = computed(
  () => [...executionLogs.logs] as unknown as LogEntry[],
);
const hasLogs = computed(() => entries.value.length > 0);
const counts = computed(() => {
  let runCount = 0;
  let errorCount = 0;
  for (const e of entries.value) {
    if (["开始计算", "计算进度", "计算完成", "计算失败"].includes(e.stage)) runCount++;
    if (e.level === "error") errorCount++;
  }
  return { runCount, errorCount };
});

function onClear() {
  if (!hasLogs.value) return;
  if (window.confirm("确定清空当前执行日志吗？（已随历史记录保存的日志不受影响）")) {
    clearLogs();
  }
}
</script>

<template>
  <Card class="border-teal-600/30">
    <CardHeader class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <CardTitle class="flex items-center gap-2">
          <ScrollText class="size-5 text-teal-600" />
          执行日志
        </CardTitle>
        <CardDescription>
          记录上传解析、计算传参、每一步进度与结果（最新在前，点击条目可展开参数与数据详情）
        </CardDescription>
      </div>
      <div class="flex items-center gap-2 self-start sm:self-auto">
        <span v-if="hasLogs" class="text-xs text-muted-foreground">
          {{ entries.length }} 条 · 计算相关 {{ counts.runCount }} 条
          <template v-if="counts.errorCount > 0"> · {{ counts.errorCount }} 条错误</template>
        </span>
        <Button
          variant="outline"
          size="sm"
          class="gap-1.5 text-muted-foreground"
          :disabled="!hasLogs"
          @click="onClear"
        >
          <Trash2 class="size-3.5" />
          清空
        </Button>
      </div>
    </CardHeader>
    <CardContent>
      <LogList v-if="hasLogs" :entries="entries" />
      <div
        v-else
        class="flex items-center justify-center gap-2 rounded-lg border border-dashed py-8 text-sm text-muted-foreground"
      >
        <ScrollText class="size-4" />
        暂无日志：上传文件或开始计算后将自动记录每一步执行过程
      </div>
    </CardContent>
  </Card>
</template>
