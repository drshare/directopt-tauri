<script setup lang="ts">
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { params } from "@/composables/useParams";

/**
 * 择优范围与遗传算法参数（对应 DR-1.3 / AR-3.3）
 * 范围字段显式绑定到 params 的起止键，避免运行时字符串拼接造成的类型断言。
 */

/** 风光储择优范围字段：显式声明 startKey / endKey，保证类型安全 */
interface RangeField {
  startKey: "windStart" | "pvStart" | "essStart";
  endKey: "windEnd" | "pvEnd" | "essEnd";
  label: string;
  unit: string;
}

const rangeFields: RangeField[] = [
  { startKey: "windStart", endKey: "windEnd", label: "风电规模", unit: "MW" },
  { startKey: "pvStart", endKey: "pvEnd", label: "光伏规模", unit: "MW" },
  { startKey: "essStart", endKey: "essEnd", label: "储能容量", unit: "MWh" },
];

/** 遗传算法参数字段 */
const gaFields: Array<{
  key: "generations" | "crossoverRate" | "mutationRate" | "populationSize";
  label: string;
}> = [
  { key: "generations", label: "遗传代数" },
  { key: "crossoverRate", label: "交叉概率" },
  { key: "mutationRate", label: "变异概率" },
  { key: "populationSize", label: "种群大小" },
];
</script>

<template>
  <div class="space-y-6">
    <div>
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">风光储容量择优范围</h3>
      <div class="grid gap-x-6 gap-y-4 sm:grid-cols-3">
        <div v-for="item in rangeFields" :key="item.startKey" class="space-y-1.5">
          <Label class="text-xs text-muted-foreground">{{ item.label }}（{{ item.unit }}）</Label>
          <div class="flex items-center gap-2">
            <Input v-model="params[item.startKey]" class="flex-1" placeholder="起始值" />
            <span class="text-muted-foreground">~</span>
            <Input v-model="params[item.endKey]" class="flex-1" placeholder="结束值" />
          </div>
          <p class="text-[11px] text-muted-foreground/80">首次计算建议取大范围以便找到可行解</p>
        </div>
      </div>
    </div>

    <Separator />

    <div>
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">遗传算法参数</h3>
      <div class="grid gap-x-6 gap-y-4 sm:grid-cols-2 xl:grid-cols-4">
        <div v-for="item in gaFields" :key="item.key" class="space-y-1.5">
          <Label class="text-xs text-muted-foreground">{{ item.label }}</Label>
          <Input v-model="params[item.key]" />
        </div>
      </div>
      <p class="mt-2 text-[11px] text-muted-foreground/80">默认：遗传代数 40 · 交叉概率 0.5 · 变异概率 0.3 · 种群大小 100</p>
    </div>
  </div>
</template>
