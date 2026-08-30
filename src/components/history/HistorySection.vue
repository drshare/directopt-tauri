<script setup lang="ts">
import { computed } from "vue";
import {
  CircleCheck,
  Clock3,
  FileSpreadsheet,
  History,
  RotateCcw,
  ScrollText,
  Trash2,
} from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { computation, restoreResult, type MetricItem } from "@/composables/useComputation";
import {
  clearHistory,
  history,
  removeHistory,
  type HistoryRecord,
} from "@/composables/useHistory";
import LogList from "@/components/log/LogList.vue";

/** 从结果头部指标卡中提取指定指标值 */
function metric(record: HistoryRecord, label: string): string {
  const found = record.result.headline.find((item) => item.label === label);
  return found?.value ?? "—";
}

/** 记录摘要：风电 / 光伏 / 储能功率 / 储能容量 / 综合电价 */
const summaryLabels = ["最优风电规模", "最优光伏规模", "最优储能功率", "最优储能容量", "综合电价"] as const;

function summaryOf(record: HistoryRecord): MetricItem[] {
  return summaryLabels.map((label) => ({ label, value: metric(record, label) }));
}

const hasHistory = computed(() => history.length > 0);

/** 载入历史记录：恢复结果并切换到“已完成”状态 */
function loadRecord(record: HistoryRecord) {
  restoreResult(record.result);
  computation.status = "done";
  computation.message = `已载入 ${record.atLabel} 的计算结果，可在下方查看并导出`;
}

function onClear() {
  if (!hasHistory.value) return;
  if (window.confirm(`确定清空全部 ${history.length} 条计算历史吗？`)) {
    clearHistory();
  }
}
</script>

<template>
  <Card class="border-sky-600/30">
    <CardHeader class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <CardTitle class="flex items-center gap-2">
          <History class="size-5 text-sky-600" />
          计算历史
        </CardTitle>
        <CardDescription>每次计算完成后自动记录参数与结果快照，可回看、载入或删除</CardDescription>
      </div>
      <Button
        variant="outline"
        size="sm"
        class="gap-1.5 self-start text-muted-foreground sm:self-auto"
        :disabled="!hasHistory"
        @click="onClear"
      >
        <Trash2 class="size-3.5" />
        清空
      </Button>
    </CardHeader>
    <CardContent>
      <!-- 空态 -->
      <div
        v-if="!hasHistory"
        class="flex items-center justify-center gap-2 rounded-lg border border-dashed py-8 text-sm text-muted-foreground"
      >
        <Clock3 class="size-4" />
        暂无计算历史，完成一次计算后自动记录
      </div>

      <!-- 历史列表 -->
      <ol v-else class="space-y-3">
        <li
          v-for="record in history"
          :key="record.id"
          class="rounded-lg border bg-muted/20 p-4"
        >
          <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="text-sm font-medium">{{ record.atLabel }}</span>
                <Badge variant="secondary" class="gap-1">
                  <FileSpreadsheet class="size-3" />
                  {{ record.curveFile || "未上传曲线" }}
                </Badge>
              </div>
              <p class="mt-1 text-xs text-muted-foreground">
                {{ record.schemeLabel }} · {{ record.objectiveLabel }} ·
                <template v-if="record.params.algorithm === 'ga'">
                  遗传算法 · 代数 {{ record.params.generations }} · 种群 {{ record.params.populationSize }}
                </template>
                <template v-else>
                  贝叶斯优化 · 总评估 {{ record.params.nIter ?? 100 }} · 初始采样
                  {{ record.params.nInit ?? 20 }}
                </template>
              </p>
              <div class="mt-2 flex flex-wrap gap-2">
                <span
                  v-for="item in summaryOf(record)"
                  :key="item.label"
                  class="rounded-md border bg-background px-2 py-1 text-xs"
                >
                  <span class="text-muted-foreground">{{ item.label }}：</span>
                  <span class="font-medium">{{ item.value }}</span>
                </span>
              </div>
              <!-- 随历史记录保存的执行日志 -->
              <details v-if="record.logs && record.logs.length > 0" class="mt-3">
                <summary
                  class="flex w-fit cursor-pointer list-none items-center gap-1.5 rounded-md border bg-background px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
                >
                  <ScrollText class="size-3.5 text-teal-600" />
                  执行日志（{{ record.logs.length }} 条，含上传解析 / 计算传参 / 各步骤详情）
                </summary>
                <div class="mt-2">
                  <LogList :entries="record.logs" :default-open="false" />
                </div>
              </details>
            </div>

            <div class="flex shrink-0 items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                class="gap-1.5"
                :disabled="computation.status === 'queued' || computation.status === 'running'"
                @click="loadRecord(record)"
              >
                <RotateCcw class="size-3.5" />
                载入
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                title="删除该记录"
                @click="removeHistory(record.id)"
              >
                <Trash2 class="size-3.5 text-muted-foreground" />
              </Button>
            </div>
          </div>
        </li>
      </ol>

      <template v-if="hasHistory">
        <Separator class="my-4" />
        <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
          <CircleCheck class="size-3.5 text-emerald-600" />
          共 {{ history.length }} 条记录（最多保留 50 条，超出自动删除最旧记录；每条记录含完整执行日志）
        </p>
      </template>
    </CardContent>
  </Card>
</template>
