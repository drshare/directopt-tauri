<script setup lang="ts">
import { CircleAlert, LoaderCircle } from "@lucide/vue";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Progress } from "@/components/ui/progress";
import { computation } from "@/composables/useComputation";
import { issues } from "@/composables/useStartCompute";
</script>

<template>
  <!-- 计算状态提示：仅排队/计算中/异常时显示（状态徽标已移至顶栏「开始」按钮旁，完成由结果区直接呈现） -->
  <div
    v-if="computation.status === 'queued' || computation.status === 'running' || computation.status === 'error'"
    class="space-y-3"
  >
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
  </div>
</template>
