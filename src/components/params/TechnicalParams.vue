<script setup lang="ts">
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { params } from "@/composables/useParams";

/**
 * 技术参数（对应输入文件模板 DR-1.1）
 * 说明：参数由上传文件读取回填；界面修改覆盖上传值
 */
/** 仅取字符串值参数键（排除 chargePeriods/dischargePeriods 等数组字段） */
type StringKey = {
  [K in keyof typeof params]: typeof params[K] extends string ? K : never;
}[keyof typeof params];

const fields: { key: StringKey; label: string; unit: string; hint?: string }[] = [
  { key: "dod", label: "储能充放电深度", unit: "%", hint: "建议 80~90（磷酸铁锂）/80（三元锂）" },
  { key: "rate", label: "电池充放电倍率", unit: "C" },
  { key: "initialSoc", label: "储能初始电量", unit: "%", hint: "与充放电深度之和 ≥ 100" },
  { key: "chargeEff", label: "储能充电效率", unit: "%", hint: "默认 93" },
  { key: "dischargeEff", label: "储能放电效率", unit: "%", hint: "默认 92" },
  { key: "gridCapacity", label: "接入公共电网容量（最大下网功率）", unit: "kW", hint: "受电变压器容量×功率因数" },
  { key: "avgLoadRate", label: "平均负荷率", unit: "%", hint: "按省份 110kV 及以上工商业两部制平均水平，电网公司定期核定" },
  { key: "selfUseGenMin", label: "自发自用占总可用发电量比例下限", unit: "%", hint: "1192 号文要求 ≥ 60" },
  { key: "selfUseLoadMin", label: "自发自用占总用电量比例下限", unit: "%", hint: "≥ 30，2030 年起 ≥ 35" },
  { key: "feedLimit", label: "余电上网比例上限", unit: "%", hint: "一般不高于 20" },
  { key: "feedPower", label: "余电最大上网功率", unit: "kW" },
  { key: "curtailLimit", label: "弃电率上限", unit: "%" },
];
</script>

<template>
  <div class="grid gap-x-6 gap-y-4 sm:grid-cols-2 xl:grid-cols-3">
    <div v-for="f in fields" :key="f.key" class="space-y-1.5">
      <Label :for="`tech-${f.key}`" class="text-xs text-muted-foreground">{{ f.label }}</Label>
      <div class="relative">
        <Input :id="`tech-${f.key}`" v-model="params[f.key]" class="pr-10" />
        <span class="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs text-muted-foreground">{{ f.unit }}</span>
      </div>
      <p v-if="f.hint" class="text-[11px] leading-tight text-muted-foreground/80">{{ f.hint }}</p>
    </div>
  </div>
</template>
