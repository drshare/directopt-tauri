<script setup lang="ts">
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { params } from "@/composables/useParams";

/**
 * 经济评价参数（对应输入文件模板 DR-1.2）
 */
/** 仅取字符串值参数键（排除数组字段） */
type StringKey = {
  [K in keyof typeof params]: typeof params[K] extends string ? K : never;
}[keyof typeof params];

const fields: { key: StringKey; label: string; unit: string; hint?: string }[] = [
  { key: "windInvest", label: "风电系统单位投资", unit: "元/kW" },
  { key: "pvInvest", label: "光伏系统单位投资", unit: "元/kW" },
  { key: "essInvest", label: "储能系统单位投资", unit: "元/kWh" },
  { key: "opexRatio", label: "年运维费用占比", unit: "%", hint: "一般取 1~3" },
  { key: "salary", label: "人员工资", unit: "万元/人年" },
  { key: "staffCount", label: "定员人数", unit: "人" },
  { key: "discountRate", label: "折现率", unit: "%", hint: "净现值为零时的收益率" },
  { key: "evalPeriod", label: "评价周期", unit: "年", hint: "一般为 15~20" },
  { key: "otherInvest", label: "其他固定投资", unit: "万元", hint: "如输电线路投资" },
  { key: "batteryReplaceUnit", label: "电池更换单价", unit: "元/kWh" },
  { key: "batteryReplaceRatio", label: "电池更换比例", unit: "%" },
  { key: "batteryReplaceYear", label: "电池更换时间", unit: "年末", hint: "填第 N 年末更换，如 8；锂电池寿命一般 5~8 年" },
];
</script>

<template>
  <div class="grid gap-x-6 gap-y-4 sm:grid-cols-2 xl:grid-cols-3">
    <div v-for="f in fields" :key="f.key" class="space-y-1.5">
      <Label :for="`eco-${f.key}`" class="text-xs text-muted-foreground">{{ f.label }}</Label>
      <div class="relative">
        <Input :id="`eco-${f.key}`" v-model="params[f.key]" class="pr-16" />
        <span class="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs text-muted-foreground">{{ f.unit }}</span>
      </div>
      <p v-if="f.hint" class="text-[11px] leading-tight text-muted-foreground/80">{{ f.hint }}</p>
    </div>
  </div>
</template>
