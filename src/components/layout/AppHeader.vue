<script setup lang="ts">
import { ref } from "vue";
import { Cloud, FolderOpen, LoaderCircle, Play, Timer } from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import FileManagerDialog from "@/components/files/FileManagerDialog.vue";
import { computation } from "@/composables/useComputation";
import { startCompute } from "@/composables/useStartCompute";

const filesOpen = ref(false);

const statusBadge: Record<string, { label: string; cls: string }> = {
  queued: { label: "排队中", cls: "bg-amber-500 text-white" },
  running: { label: "计算中", cls: "bg-sky-500 text-white" },
  done: { label: "已完成", cls: "bg-emerald-500 text-white" },
  error: { label: "校验未通过", cls: "bg-destructive text-white" },
};

function handleFiles() {
  filesOpen.value = true;
}
</script>

<template>
  <header class="sticky top-0 z-20 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
    <div class="flex h-14 w-full items-center gap-2 px-3 sm:h-16 sm:gap-4 sm:px-4 lg:px-6">
      <div class="flex min-w-0 items-center gap-2 sm:gap-3">
        <div class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-emerald-600 text-white">
          <Cloud class="size-5" aria-hidden="true" />
        </div>
        <div class="min-w-0 leading-tight">
          <div class="flex min-w-0 items-center gap-2">
            <h1 class="truncate text-sm font-semibold tracking-tight sm:text-base">绿电直连新能源优化配置软件</h1>
            <Badge variant="outline" class="shrink-0 text-xs font-normal text-muted-foreground">V2.2</Badge>
          </div>
          <p class="hidden truncate text-xs text-muted-foreground sm:block">风电 · 光伏 · 储能最优容量配置计算</p>
        </div>
      </div>

      <div class="ml-auto flex shrink-0 items-center gap-2 sm:gap-3">
        <Button
          variant="ghost"
          size="sm"
          class="gap-1 text-muted-foreground"
          aria-label="文件管理"
          @click="handleFiles"
        >
          <FolderOpen class="size-4" aria-hidden="true" />
          <span class="hidden sm:inline">文件管理</span>
        </Button>
        <!-- 状态徽标：仅在非空闲时显示（idle 状态不占空间，保持顶栏简洁） -->
        <Badge
          v-if="computation.status !== 'idle'"
          :class="statusBadge[computation.status]?.cls"
          class="shrink-0"
        >
          {{ statusBadge[computation.status]?.label }}
        </Badge>
        <Button
          size="sm"
          class="min-w-24 gap-1.5"
          :disabled="computation.status === 'queued' || computation.status === 'running'"
          aria-label="开始计算"
          @click="startCompute"
        >
          <LoaderCircle v-if="computation.status === 'running'" class="animate-spin" />
          <Timer v-else-if="computation.status === 'queued'" />
          <Play v-else />
          {{ computation.status === "queued" || computation.status === "running" ? "计算中…" : "开始" }}
        </Button>
      </div>
    </div>
  </header>
  <FileManagerDialog v-model:open="filesOpen" />
</template>
