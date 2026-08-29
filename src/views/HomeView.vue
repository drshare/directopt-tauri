<script setup lang="ts">
import AppHeader from "@/components/layout/AppHeader.vue";
import UploadSection from "@/components/upload/UploadSection.vue";
import ParameterSection from "@/components/params/ParameterSection.vue";
import ComputeControl from "@/components/compute/ComputeControl.vue";
import HistorySection from "@/components/history/HistorySection.vue";
import ResultSection from "@/components/result/ResultSection.vue";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
</script>

<template>
  <div class="min-h-dvh bg-muted/30">
    <AppHeader />
    <main class="w-full space-y-4 p-3 sm:space-y-6 sm:p-4 lg:px-6">
      <!-- 计算控制 + 数据准备与上传（合并卡片，紧凑无大标题） -->
      <Card>
        <CardContent class="space-y-4">
          <ComputeControl />
          <Separator />
          <UploadSection />
        </CardContent>
      </Card>

      <!-- 参数输入与确认 -->
      <ParameterSection />

      <!-- 计算历史 -->
      <HistorySection />

      <!-- 计算结果 -->
      <ResultSection />
    </main>
    <footer
      class="w-full px-3 pb-[max(1.5rem,env(safe-area-inset-bottom))] pt-2 text-center text-xs text-muted-foreground sm:px-4 lg:px-6"
    >
      绿电直连新能源优化配置软件 V2.2 · 算法符合发改能源〔2025〕650 号、发改价格〔2025〕1192 号 · 技术支持 1410789590@qq.com
    </footer>
  </div>
</template>
