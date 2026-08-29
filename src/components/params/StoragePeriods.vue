<script setup lang="ts">
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { params } from "@/composables/useParams";

/**
 * 储能充电优先时段 / 储能允许放电时段（仅可在软件界面操作，DR-1.1）
 * 24 个时段（0:00~24:00）
 */
const PERIODS = Array.from({ length: 24 }, (_, i) => {
  const start = `${String(i).padStart(2, "0")}:00`;
  const end = `${String(i + 1).padStart(2, "0")}:00`;
  return { id: i, label: `${start}-${end}` };
});

function toggle(list: number[], id: number): number[] {
  return list.includes(id) ? list.filter((x) => x !== id) : [...list, id];
}
</script>

<template>
  <div class="grid gap-6 lg:grid-cols-2">
    <div>
      <div class="mb-2 flex items-center justify-between">
        <Label class="text-sm font-medium">储能充电优先时段</Label>
        <span class="text-xs text-muted-foreground">已选 {{ params.chargePeriods.length }} 个时段</span>
      </div>
      <p class="mb-3 text-xs text-muted-foreground">
        勾选的时段，风光发电优先用于给储能充电，有余电再供负荷（用于网电价格较低时段）
      </p>
      <div class="grid grid-cols-3 gap-1.5 rounded-lg border p-3 sm:grid-cols-4">
        <label
          v-for="p in PERIODS"
          :key="`c-${p.id}`"
          class="flex cursor-pointer items-center gap-1.5 rounded px-1 py-1 text-xs hover:bg-muted"
        >
          <Checkbox
            :model-value="params.chargePeriods.includes(p.id)"
            @update:model-value="params.chargePeriods = toggle(params.chargePeriods, p.id)"
          />
          {{ p.label }}
        </label>
      </div>
    </div>

    <Separator class="lg:hidden" />

    <div>
      <div class="mb-2 flex items-center justify-between">
        <Label class="text-sm font-medium">储能允许放电时段</Label>
        <span class="text-xs text-muted-foreground">已选 {{ params.dischargePeriods.length }} 个时段</span>
      </div>
      <p class="mb-3 text-xs text-muted-foreground">
        勾选的时段允许储能放电（用于网电价格较高时段）
      </p>
      <div class="grid grid-cols-3 gap-1.5 rounded-lg border p-3 sm:grid-cols-4">
        <label
          v-for="p in PERIODS"
          :key="`d-${p.id}`"
          class="flex cursor-pointer items-center gap-1.5 rounded px-1 py-1 text-xs hover:bg-muted"
        >
          <Checkbox
            :model-value="params.dischargePeriods.includes(p.id)"
            @update:model-value="params.dischargePeriods = toggle(params.dischargePeriods, p.id)"
          />
          {{ p.label }}
        </label>
      </div>
    </div>
  </div>
</template>
