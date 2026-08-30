<script setup lang="ts">
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Separator } from "@/components/ui/separator";
import { params } from "@/composables/useParams";

/**
 * 择优范围与算法参数
 * - V3.0 口径：贝叶斯优化（总评估次数 / 初始随机采样点数），默认
 * - V2.2 口径：遗传算法（遗传代数 / 交叉概率 / 变异概率 / 种群大小）
 *
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

/** 寻优算法选项 */
const algorithmOptions: Array<{ value: "bo" | "ga"; label: string; hint: string }> = [
  {
    value: "bo",
    label: "贝叶斯优化",
    hint: "V3.0 口径 · 高斯过程代理模型 + 期望改善，评估次数少、收敛快",
  },
  {
    value: "ga",
    label: "遗传算法",
    hint: "V2.2 说明书口径 · 锦标赛选择 + SBX 交叉 + 高斯变异",
  },
];

/** 贝叶斯优化参数字段（V3.0） */
const boFields: Array<{
  key: "nIter" | "nInit";
  label: string;
  placeholder: string;
}> = [
  { key: "nIter", label: "总评估次数", placeholder: "100" },
  { key: "nInit", label: "初始随机采样点数", placeholder: "20" },
];

/** 遗传算法参数字段（V2.2） */
const gaFields: Array<{
  key: "generations" | "crossoverRate" | "mutationRate" | "populationSize";
  label: string;
}> = [
  { key: "generations", label: "遗传代数" },
  { key: "crossoverRate", label: "交叉概率" },
  { key: "mutationRate", label: "变异概率" },
  { key: "populationSize", label: "种群大小" },
];

/** 当前算法下的一次完整计算的仿真次数（用于提示计算量） */
const evalHint = () => {
  if (params.algorithm === "ga") {
    const pop = Number(params.populationSize) || 0;
    const gen = Number(params.generations) || 0;
    return `约 ${(pop * (gen + 1)).toLocaleString("zh-CN")} 次 8760h 仿真`;
  }
  return `${Number(params.nIter) || 0} 次 8760h 仿真`;
};
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
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">寻优算法</h3>
      <RadioGroup v-model="params.algorithm" class="grid gap-3 sm:grid-cols-2">
        <label
          v-for="opt in algorithmOptions"
          :key="opt.value"
          class="flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition-colors hover:bg-accent"
          :class="params.algorithm === opt.value ? 'border-primary bg-accent/50' : ''"
        >
          <RadioGroupItem :value="opt.value" class="mt-0.5" />
          <span class="space-y-1">
            <span class="block text-sm font-medium">{{ opt.label }}</span>
            <span class="block text-[11px] leading-relaxed text-muted-foreground">{{ opt.hint }}</span>
          </span>
        </label>
      </RadioGroup>
    </div>

    <div v-if="params.algorithm === 'bo'">
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">算法参数（贝叶斯优化）</h3>
      <div class="grid gap-x-6 gap-y-4 sm:grid-cols-2 xl:grid-cols-4">
        <div v-for="item in boFields" :key="item.key" class="space-y-1.5">
          <Label class="text-xs text-muted-foreground">{{ item.label }}</Label>
          <Input v-model="params[item.key]" :placeholder="item.placeholder" />
        </div>
      </div>
      <p class="mt-2 text-[11px] text-muted-foreground/80">
        默认：总评估次数 100 · 初始随机采样点数 20（与 V3.0 输入模板一致）；当前计算量 {{ evalHint() }}
      </p>
    </div>

    <div v-else>
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">算法参数（遗传算法）</h3>
      <div class="grid gap-x-6 gap-y-4 sm:grid-cols-2 xl:grid-cols-4">
        <div v-for="item in gaFields" :key="item.key" class="space-y-1.5">
          <Label class="text-xs text-muted-foreground">{{ item.label }}</Label>
          <Input v-model="params[item.key]" />
        </div>
      </div>
      <p class="mt-2 text-[11px] text-muted-foreground/80">
        默认：遗传代数 40 · 交叉概率 0.5 · 变异概率 0.3 · 种群大小 100；当前计算量 {{ evalHint() }}
      </p>
    </div>
  </div>
</template>
