<script setup lang="ts">
import { ref } from "vue";
import { CircleAlert, CircleCheck, Play, LoaderCircle, Timer } from "@lucide/vue";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { computation, result, runComputation } from "@/composables/useComputation";
import {
  OBJECTIVE_LABELS,
  SCHEME_LABELS,
  params,
  snapshotParams,
  validateParams,
  type ParamIssue,
} from "@/composables/useParams";
import { addHistory } from "@/composables/useHistory";

const issues = ref<ParamIssue[]>([]);

async function startCompute() {
  if (computation.status === "queued" || computation.status === "running") return;

  // FR-5 参数合理性校验：不通过则阻止进入计算
  issues.value = validateParams();
  if (issues.value.length > 0) {
    computation.status = "error";
    computation.message = `参数校验未通过，共 ${issues.value.length} 项，请修正后重试`;
    return;
  }

  // 启动后端计算（浏览器环境自动回退为演示流程）
  await runComputation();

  // 计算完成后写入历史记录（参数快照 + 结果快照）
  addHistory({
    curveFile: params.curveFile,
    schemeLabel: SCHEME_LABELS[params.scheme],
    objectiveLabel: OBJECTIVE_LABELS[params.objective],
    params: snapshotParams(),
    result: {
      headline: result.headline.map((item) => ({ ...item })),
      invest: result.invest.map((item) => ({ ...item })),
      opex: result.opex.map((item) => ({ ...item })),
    },
  });
}

const statusBadge: Record<string, { label: string; cls: string }> = {
  idle: { label: "未开始", cls: "bg-muted text-muted-foreground" },
  queued: { label: "排队中", cls: "bg-amber-500 text-white" },
  running: { label: "计算中", cls: "bg-sky-500 text-white" },
  done: { label: "已完成", cls: "bg-emerald-500 text-white" },
  error: { label: "参数校验未通过", cls: "bg-destructive text-white" },
};
</script>

<template>
  <div class="space-y-3">
    <!-- 紧凑控制行：按钮 + 状态 + 提示 -->
    <div class="flex flex-wrap items-center gap-3">
      <Button
        type="button"
        class="min-w-28 gap-2"
        :disabled="computation.status === 'queued' || computation.status === 'running'"
        @click="startCompute"
      >
        <LoaderCircle v-if="computation.status === 'running'" class="animate-spin" />
        <Timer v-else-if="computation.status === 'queued'" />
        <Play v-else />
        {{ computation.status === "queued" || computation.status === "running" ? "计算中…" : "开始" }}
      </Button>
      <Badge :class="statusBadge[computation.status].cls">
        {{ statusBadge[computation.status].label }}
      </Badge>
      <p v-if="computation.status === 'idle'" class="min-w-0 flex-1 text-xs text-muted-foreground">
        参数确认无误后点击“开始”，先进行参数校验再启动优化算法（遗传算法求解）
      </p>
    </div>

    <!-- 排队 -->
    <Alert
      v-if="computation.status === 'queued'"
      variant="default"
      class="border-amber-400/50 bg-amber-50 text-amber-900"
    >
      <CircleAlert class="size-4 text-amber-600" />
      <AlertTitle>任务排队中</AlertTitle>
      <AlertDescription>当前队列共有 {{ computation.queueCount }} 个任务等待执行，请耐心等待。</AlertDescription>
    </Alert>

    <!-- 计算中 -->
    <template v-else-if="computation.status === 'running'">
      <Alert variant="default" class="border-sky-400/50 bg-sky-50 text-sky-900">
        <LoaderCircle class="size-4 animate-spin text-sky-600" />
        <AlertTitle>计算中</AlertTitle>
        <AlertDescription>计算耗时一般 3~5 分钟，请勿关闭页面，否则无法显示计算结果。</AlertDescription>
      </Alert>
      <div class="space-y-1">
        <Progress :model-value="computation.progress" class="h-2" />
        <p class="text-right text-xs tabular-nums text-muted-foreground">{{ computation.progress }}%</p>
      </div>
    </template>

    <!-- 参数校验失败 / 计算异常 -->
    <Alert v-else-if="computation.status === 'error'" variant="destructive">
      <CircleAlert class="size-4" />
      <AlertTitle>{{ issues.length > 0 ? `参数校验未通过（${issues.length} 项）` : "计算失败" }}</AlertTitle>
      <AlertDescription>
        <template v-if="issues.length > 0">
          <ul class="mt-2 list-inside list-disc space-y-1">
            <li v-for="(it, i) in issues" :key="i">{{ it.message }}</li>
          </ul>
        </template>
        <template v-else>{{ computation.message }}</template>
      </AlertDescription>
    </Alert>

    <!-- 计算完成 -->
    <Alert
      v-else-if="computation.status === 'done'"
      variant="default"
      class="border-emerald-400/50 bg-emerald-50 text-emerald-900"
    >
      <CircleCheck class="size-4 text-emerald-600" />
      <AlertTitle>计算完成</AlertTitle>
      <AlertDescription>
        可在下方结果区查看最优配置、指标、敏感性分析与 8760h 运行曲线，并导出 Excel 报告。
      </AlertDescription>
    </Alert>
  </div>
</template>
