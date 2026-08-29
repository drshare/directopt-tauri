<script setup lang="ts">
import { Cloud, FolderOpen } from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

function handleFiles() {
  // TODO: 文件管理（FR-10）——查看/删除历史计算输入文件与计算结果，待对接平台接口
  window.alert("文件管理功能待实现（查看/删除历史输入文件与计算结果）");
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
      </div>
    </div>
  </header>
</template>
