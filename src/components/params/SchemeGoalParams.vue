<script setup lang="ts">
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Separator } from "@/components/ui/separator";
import { params } from "@/composables/useParams";

/**
 * 输配电费缴纳方案 + 优化目标选择（仅可在软件界面操作，DR-1.4 / AR-1）
 */
const schemes = [
  {
    value: "scheme1" as const,
    title: "方案一",
    desc: "输配电费按“所在电压等级现行电度输配电价 × 平均负荷率 × 730h × 接入公共电网容量”缴纳，与实际用电量无关（适用于常规项目）。",
  },
  {
    value: "scheme2" as const,
    title: "方案二",
    desc: "可靠性要求高、需进行容量备份的项目（如 A 级数据中心 2N 供电），输配电费根据“实际用电量（含自发自用电量）× 所在电压等级现行电度输配电价”缴纳。",
  },
];

const objectives = [
  {
    value: "composite" as const,
    title: "综合电价最低",
    desc: "考虑风光储投资和运行费用、外购网电成本后的综合电价最低",
  },
  {
    value: "green" as const,
    title: "绿电电价最低",
    desc: "考虑风光储投资和运行费用后的绿电电价最低",
  },
  {
    value: "capex" as const,
    title: "初投资最低",
    desc: "风光储初投资最低",
  },
];
</script>

<template>
  <div class="space-y-6">
    <div>
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">输配电费缴纳方案</h3>
      <RadioGroup v-model="params.scheme" class="grid gap-3 lg:grid-cols-2">
        <label
          v-for="s in schemes"
          :key="s.value"
          class="flex cursor-pointer items-start gap-3 rounded-lg border p-4 transition-colors has-data-[state=checked]:border-emerald-600/60 has-data-[state=checked]:bg-emerald-50/60"
        >
          <RadioGroupItem :value="s.value" class="mt-0.5" />
          <div>
            <div class="text-sm font-medium">{{ s.title }}</div>
            <p class="mt-1 text-xs leading-relaxed text-muted-foreground">{{ s.desc }}</p>
          </div>
        </label>
      </RadioGroup>
    </div>

    <Separator />

    <div>
      <h3 class="mb-3 text-sm font-medium text-muted-foreground">优化目标选择</h3>
      <RadioGroup v-model="params.objective" class="grid gap-3 lg:grid-cols-3">
        <label
          v-for="o in objectives"
          :key="o.value"
          class="flex cursor-pointer items-start gap-3 rounded-lg border p-4 transition-colors has-data-[state=checked]:border-emerald-600/60 has-data-[state=checked]:bg-emerald-50/60"
        >
          <RadioGroupItem :value="o.value" class="mt-0.5" />
          <div>
            <div class="text-sm font-medium">{{ o.title }}</div>
            <p class="mt-1 text-xs leading-relaxed text-muted-foreground">{{ o.desc }}</p>
          </div>
        </label>
      </RadioGroup>
    </div>
  </div>
</template>
