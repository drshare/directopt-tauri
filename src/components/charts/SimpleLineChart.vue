<script setup lang="ts">
import { computed } from "vue";

export interface ChartSeries {
  name: string;
  color: string;
  values: number[];
}

const props = withDefaults(
  defineProps<{
    series: ChartSeries[];
    height?: number;
    xLabels?: string[];
  }>(),
  { height: 200, xLabels: () => [] },
);

const W = 600;
const H = computed(() => props.height);
const PAD_X = 44;
const PAD_Y = 24;

const all = computed(() => props.series.flatMap((s) => s.values));
const min = computed(() => (all.value.length ? Math.min(...all.value) : 0));
const max = computed(() => (all.value.length ? Math.max(...all.value) : 1));
const range = computed(() => (max.value - min.value || 1));

const innerW = computed(() => W - PAD_X * 2);
const innerH = computed(() => H.value - PAD_Y * 2);

function x(i: number, count: number): number {
  return PAD_X + (count <= 1 ? 0 : (i / (count - 1)) * innerW.value);
}
function y(v: number): number {
  return PAD_Y + innerH.value - ((v - min.value) / range.value) * innerH.value;
}

/** 网格线 y 坐标与刻度值 */
const gridLines = computed(() => {
  const n = 4;
  return Array.from({ length: n + 1 }, (_, i) => {
    const v = max.value - (range.value * i) / n;
    return { v, y: y(v) };
  });
});

const maxCount = computed(() => Math.max(...props.series.map((s) => s.values.length), 1));
const xTickIdx = computed(() => {
  const n = Math.min(maxCount.value, 6);
  return Array.from({ length: n }, (_, i) => Math.round((i / (n - 1)) * (maxCount.value - 1)));
});
</script>

<template>
  <div class="w-full">
    <!-- 图例 -->
    <div class="mb-2 flex flex-wrap items-center gap-x-4 gap-y-1">
      <span v-for="s in series" :key="s.name" class="flex items-center gap-1.5 text-xs text-muted-foreground">
        <span class="inline-block h-2 w-3 rounded-sm" :style="{ backgroundColor: s.color }" />
        {{ s.name }}
      </span>
    </div>

    <svg :viewBox="`0 0 ${W} ${H}`" class="w-full" preserveAspectRatio="none" role="img">
      <!-- 网格线与 y 轴刻度 -->
      <g v-for="(g, i) in gridLines" :key="i">
        <line :x1="PAD_X" :x2="W - PAD_X" :y1="g.y" :y2="g.y" class="stroke-border" stroke-width="1" stroke-dasharray="4 4" />
        <text :x="PAD_X - 6" :y="g.y + 3" text-anchor="end" class="fill-muted-foreground" font-size="10">
          {{ g.v.toFixed(0) }}
        </text>
      </g>
      <!-- x 轴刻度 -->
      <g v-for="i in xTickIdx" :key="`x-${i}`">
        <text :x="x(i, maxCount)" :y="H - 6" text-anchor="middle" class="fill-muted-foreground" font-size="10">
          {{ xLabels[i] ?? `${i}` }}
        </text>
      </g>
      <!-- 折线 -->
      <g v-for="s in series" :key="s.name">
        <polyline
          :points="s.values.map((v, i) => `${x(i, s.values.length)},${y(v)}`).join(' ')"
          :fill="'none'"
          :stroke="s.color"
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
        <!-- 数据点标记（离散数据时展示） -->
        <template v-if="maxCount <= 15">
          <circle
            v-for="(v, i) in s.values"
            :key="i"
            :cx="x(i, s.values.length)"
            :cy="y(v)"
            r="3"
            :fill="s.color"
          />
        </template>
      </g>
    </svg>
  </div>
</template>
