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
import CalcDocPanel from "./CalcDocPanel.vue";
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
const RUN_STAGES = [
  "开始计算",
  "仿真初始化",
  "GA 寻优",
  "最优方案仿真",
  "敏感性分析",
  "结果汇总",
  "计算进度",
  "计算完成",
  "计算失败",
];
const counts = computed(() => {
  let runCount = 0;
  let errorCount = 0;
  for (const e of entries.value) {
    if (RUN_STAGES.includes(e.stage)) runCount++;
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
          执行日志 · 计算过程文档
        </CardTitle>
        <CardDescription>
          左栏记录上传解析、每阶段进度与结果（按时间顺序排列，最新日志自动滚动到底部；点击条目展开详情）；右栏为详细计算过程文档，与日志阶段一一对应
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
      <!-- 左右分栏：左 = 执行日志，右 = 计算过程文档（窄屏上下堆叠） -->
      <div class="grid min-h-0 items-stretch gap-3 lg:h-[30rem] lg:grid-cols-2">
        <div class="min-h-0">
          <LogList v-if="hasLogs" :entries="entries" fill />
          <div
            v-else
            class="flex h-full min-h-32 items-center justify-center gap-2 rounded-lg border border-dashed py-8 text-sm text-muted-foreground"
          >
            <ScrollText class="size-4" />
            暂无日志：上传文件或开始计算后将自动记录每一步执行过程
          </div>
        </div>
        <div class="min-h-40 lg:min-h-0">
          <CalcDocPanel />
        </div>
      </div>
    </CardContent>
  </Card>
</template>
