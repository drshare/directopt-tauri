<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, type ComponentPublicInstance } from "vue";
import { toPng } from "html-to-image";
import { use } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import { LineChart as EChartsLine } from "echarts/charts";
import {
  DataZoomComponent,
  GridComponent,
  LegendComponent,
  TooltipComponent,
} from "echarts/components";
import type { EChartsCoreOption } from "echarts/core";
import VChart from "vue-echarts";
import {
  AreaChart,
  BadgeCheck,
  Camera,
  Download,
  FileSpreadsheet,
  Gauge,
  LineChart,
  Wallet,
} from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { computation, result } from "@/composables/useComputation";
import {
  balanceSeries as BALANCE_SERIES,
  energyStats,
  hourLabels as HOUR_LABELS,
  ratioLabels as SENSITIVITY_RATIO_LABELS,
  sensitivityGroups,
  type SensitivityGroup,
} from "@/composables/useResultData";
import { exportResultWorkbook } from "@/lib/resultExport";

/** 注册 ECharts 按需模块（折线图 + 网格 / 图例 / 提示框 / 缩放组件 + Canvas 渲染器） */
use([CanvasRenderer, EChartsLine, GridComponent, LegendComponent, TooltipComponent, DataZoomComponent]);
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

const ready = computed(() => computation.status === "done");

/** 结果预览：截图结果卡片 → 图片弹窗展示 */
const resultCardRef = ref<ComponentPublicInstance | null>(null);
const previewOpen = ref(false);
const previewImage = ref("");
const previewGenerating = ref(false);

async function handlePreview() {
  const el = resultCardRef.value?.$el as HTMLElement | null;
  if (!el) return;
  previewGenerating.value = true;
  try {
    previewImage.value = await toPng(el, {
      pixelRatio: 2,
      backgroundColor: "#ffffff",
      // 截图时排除带 no-capture 标记的操作按钮
      filter: (node) => !(node instanceof HTMLElement && node.classList?.contains("no-capture")),
    });
    previewOpen.value = true;
  } catch (err) {
    console.error("生成结果预览图失败", err);
    window.alert("生成结果预览图失败，请重试");
  } finally {
    previewGenerating.value = false;
  }
}

/** 敏感性分析表格与曲线数据（与导出报告共用同一数据源；计算完成后读取最新模块级数据） */
const sensitivity = computed<SensitivityGroup[]>(() => {
  void computation.status; // 追踪计算状态：计算完成/重算后刷新
  return sensitivityGroups;
});

const ratioLabels = SENSITIVITY_RATIO_LABELS;

/** 深色模式自适应（跟随 html 元素 class 变化） */
const isDark = ref(document.documentElement.classList.contains("dark"));
let darkObserver: MutationObserver | null = null;
onMounted(() => {
  darkObserver = new MutationObserver(() => {
    isDark.value = document.documentElement.classList.contains("dark");
  });
  darkObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
});
onUnmounted(() => darkObserver?.disconnect());

const chartColors = computed(() =>
  isDark.value
    ? { text: "#94a3b8", split: "#334155", tooltipBg: "#1e293b" }
    : { text: "#64748b", split: "#e2e8f0", tooltipBg: "#ffffff" },
);

/** 电量平衡曲线缩放窗口（小时序号 0 ~ 8759），由 dataZoom 事件回写 */
const HOUR_MAX = 8759;
const zoomStart = ref(0);
const zoomEnd = ref(HOUR_MAX);

/** 图表固定高度（px），不随容器/内容变化 */
const SENSITIVITY_CHART_HEIGHT = 190;
const BALANCE_CHART_HEIGHT = 320;

/** 千分位数值格式（y 轴刻度） */
function fmtThousand(v: number): string {
  return v.toLocaleString("zh-CN", { maximumFractionDigits: 1 });
}

/** 单组敏感性曲线：变动比例 → 适应度 */
function sensitivityOption(group: SensitivityGroup): EChartsCoreOption {
  const c = chartColors.value;
  return {
    animation: false,
    grid: { left: 8, right: 16, top: 24, bottom: 6, containLabel: true },
    tooltip: {
      trigger: "axis",
      confine: true,
      backgroundColor: c.tooltipBg,
      borderColor: c.split,
      borderRadius: 6,
      padding: [6, 10],
      textStyle: { color: c.text, fontSize: 11 },
      valueFormatter: (v: unknown) => Number(v).toFixed(6),
    },
    xAxis: {
      type: "category",
      data: ratioLabels,
      boundaryGap: false,
      axisLabel: { color: c.text, fontSize: 10, hideOverlap: true },
      axisLine: { lineStyle: { color: c.split } },
      axisTick: { show: false },
    },
    yAxis: {
      type: "value",
      scale: true,
      splitNumber: 4,
      axisLabel: { color: c.text, fontSize: 10, formatter: (v: number) => v.toFixed(4) },
      splitLine: { lineStyle: { color: c.split, type: "dashed" } },
    },
    series: [
      {
        name: `${group.element}变动`,
        type: "line",
        data: group.rows.map((r) => Number(r.fitness)),
        lineStyle: { color: group.color, width: 2 },
        itemStyle: { color: group.color, borderColor: c.tooltipBg, borderWidth: 1 },
        symbol: "circle",
        symbolSize: 6,
        areaStyle: {
          color: {
            type: "linear",
            x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: `${group.color}33` },
              { offset: 1, color: `${group.color}00` },
            ],
          },
        },
      },
    ],
  };
}

/** 全年 8760h 电量平衡曲线（数据全量传入，展示窗口由 dataZoom 控制） */
const balanceOption = computed<EChartsCoreOption>(() => {
  const c = chartColors.value;
  void computation.status; // 计算完成后重新读取模块级 balanceSeries
  return {
    animation: false,
    grid: { left: 8, right: 16, top: 36, bottom: 34, containLabel: true },
    legend: {
      type: "scroll",
      top: 0,
      itemWidth: 14,
      itemHeight: 4,
      itemGap: 12,
      icon: "rect",
      textStyle: { color: c.text, fontSize: 11 },
      pageIconColor: c.text,
      pageIconInactiveColor: c.split,
      pageTextStyle: { color: c.text },
    },
    tooltip: {
      trigger: "axis",
      confine: true,
      backgroundColor: c.tooltipBg,
      borderColor: c.split,
      borderRadius: 6,
      padding: [6, 10],
      textStyle: { color: c.text, fontSize: 11 },
      valueFormatter: (v: unknown) => `${fmtThousand(Number(v))} kWh`,
    },
    xAxis: {
      type: "category",
      data: HOUR_LABELS,
      boundaryGap: false,
      axisLabel: { color: c.text, fontSize: 10, hideOverlap: true },
      axisLine: { lineStyle: { color: c.split } },
      axisTick: { show: false },
    },
    yAxis: {
      type: "value",
      scale: true,
      splitNumber: 5,
      axisLabel: { color: c.text, fontSize: 10, formatter: fmtThousand },
      splitLine: { lineStyle: { color: c.split, type: "dashed" } },
    },
    dataZoom: [
      { type: "inside", startValue: zoomStart.value, endValue: zoomEnd.value },
      {
        type: "slider",
        height: 18,
        bottom: 4,
        startValue: zoomStart.value,
        endValue: zoomEnd.value,
        borderColor: c.split,
        fillerColor: isDark.value ? "#33415599" : "#e2e8f099",
        handleStyle: { color: c.text },
        textStyle: { color: c.text, fontSize: 10 },
      },
    ],
    series: BALANCE_SERIES.map((s) => ({
      name: s.name,
      type: "line",
      data: s.values,
      showSymbol: false,
      sampling: "lttb",
      lineStyle: { color: s.color, width: 1.5 },
      itemStyle: { color: s.color },
      emphasis: { focus: "series", lineStyle: { width: 2.5 } },
    })),
  };
});

/** dataZoom 交互 → 回写当前显示的小时窗口（用于文字提示） */
function onBalanceZoom(raw: unknown) {
  const params = raw as {
    start?: number;
    end?: number;
    startValue?: unknown;
    endValue?: unknown;
  };
  const toIdx = (v: unknown, pct: number | undefined): number | undefined => {
    if (typeof v === "number" && Number.isFinite(v)) return Math.round(v);
    if (typeof pct === "number") return Math.round((pct / 100) * HOUR_MAX);
    return undefined;
  };
  const s = toIdx(params.startValue, params.start);
  const e = toIdx(params.endValue, params.end);
  if (s !== undefined) zoomStart.value = Math.max(0, Math.min(HOUR_MAX, s));
  if (e !== undefined) zoomEnd.value = Math.max(0, Math.min(HOUR_MAX, e));
}

function resetZoom() {
  zoomStart.value = 0;
  zoomEnd.value = HOUR_MAX;
}

const exporting = ref(false);

/** 下载计算结果报告（输入数据 + 输入曲线 + 输出数据 + 敏感性分析 + 逐时电量平衡） */
async function handleDownload() {
  if (exporting.value) return;
  exporting.value = true;
  try {
    const saved = await exportResultWorkbook();
    window.alert(`计算结果已导出：${saved}`);
  } catch (err) {
    console.error("导出计算结果失败", err);
    window.alert(`导出计算结果失败：${err instanceof Error ? err.message : String(err)}`);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <Card ref="resultCardRef">
    <CardHeader class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <CardTitle class="flex items-center gap-2">
          <AreaChart class="size-5 text-emerald-600" />
          计算结果
        </CardTitle>
        <CardDescription>
          <template v-if="ready">展示最优配置、电量指标、投资成本、敏感性分析与 8760h 运行曲线</template>
          <template v-else>点击上方“开始”按钮后，将在此展示优化结果</template>
        </CardDescription>
      </div>
      <div class="no-capture flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          class="gap-2"
          :disabled="!ready"
          @click="handlePreview"
        >
          <Camera class="size-4" />
          {{ previewGenerating ? "预览中…" : "预览" }}
        </Button>
        <Button :disabled="!ready || exporting" class="gap-2" @click="handleDownload">
          <Download class="size-4" />
          {{ exporting ? "导出中…" : "导出" }}
        </Button>
      </div>
    </CardHeader>
    <CardContent class="space-y-6">
      <template v-if="ready">
      <!-- 最优配置结果 -->
      <section>
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium">
          <BadgeCheck class="size-4 text-emerald-600" />
          最优配置结果
        </h3>
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          <div
            v-for="item in result.headline"
            :key="item.label"
            class="flex items-center justify-between gap-3 rounded-lg border bg-muted/30 px-4 py-3"
          >
            <span class="text-xs text-muted-foreground">{{ item.label }}</span>
            <span class="whitespace-nowrap text-sm font-semibold tabular-nums">{{ item.value }}</span>
          </div>
        </div>
      </section>

      <Separator />

      <!-- 全年电量指标 -->
      <section>
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium">
          <Gauge class="size-4 text-sky-600" />
          全年电量指标
        </h3>
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          <div
            v-for="item in energyStats"
            :key="item.label"
            class="flex items-center justify-between gap-3 rounded-lg border px-4 py-2.5"
          >
            <span class="text-xs text-muted-foreground">{{ item.label }}</span>
            <span class="whitespace-nowrap text-sm font-medium tabular-nums">{{ item.value }}</span>
          </div>
        </div>
      </section>

      <Separator />

      <!-- 投资与运行成本构成 -->
      <section>
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium">
          <Wallet class="size-4 text-amber-600" />
          投资与运行成本构成
        </h3>
        <div class="grid gap-4 lg:grid-cols-2">
          <div class="rounded-lg border">
            <div class="border-b bg-muted/40 px-4 py-2 text-sm font-medium">投资构成（万元）</div>
            <div class="divide-y">
              <div
                v-for="item in result.invest"
                :key="item.label"
                class="flex items-center justify-between px-4 py-2 text-sm"
              >
                <span class="text-muted-foreground">{{ item.label }}</span>
                <span class="font-medium">{{ item.value.replace(" 万元", "") }}</span>
              </div>
            </div>
          </div>
          <div class="rounded-lg border">
            <div class="border-b bg-muted/40 px-4 py-2 text-sm font-medium">年运行成本构成（万元）</div>
            <div class="divide-y">
              <div
                v-for="item in result.opex"
                :key="item.label"
                class="flex items-center justify-between px-4 py-2 text-sm"
              >
                <span class="text-muted-foreground">{{ item.label }}</span>
                <span class="font-medium">{{ item.value.replace(" 万元", "") }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <Separator />

      <!-- 敏感性分析 -->
      <section>
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium">
          <LineChart class="size-4 text-violet-600" />
          敏感性分析（固定两要素 · 变动单一要素，±25% / 步长 5%）
        </h3>
        <div class="grid items-start gap-4 md:grid-cols-2 xl:grid-cols-3">
          <div v-for="group in sensitivity" :key="group.group" class="overflow-hidden rounded-lg border">
            <div class="border-b bg-muted/40 px-4 py-2 text-sm font-medium">{{ group.group }}</div>
            <Table>
              <TableHeader>
                <TableRow class="bg-blue-600 hover:bg-blue-600">
                  <TableHead class="w-20 text-white">变动比例</TableHead>
                  <TableHead class="text-white">{{ group.element }}（{{ group.unit }}）</TableHead>
                  <TableHead class="text-white">适应度</TableHead>
                  <TableHead class="text-right text-white">备注</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                <TableRow v-for="row in group.rows" :key="row.ratio">
                  <TableCell class="font-mono text-xs">{{ row.ratio }}</TableCell>
                  <TableCell class="font-mono text-xs">{{ row.scale }}</TableCell>
                  <TableCell class="font-mono text-xs">{{ row.fitness }}</TableCell>
                  <TableCell class="text-right">
                    <Badge :variant="row.ok ? 'secondary' : 'destructive'" class="text-[10px]">
                      {{ row.note }}
                    </Badge>
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </div>
        </div>
        <!-- 敏感性曲线 -->
        <div class="mt-4 grid gap-4 xl:grid-cols-3">
          <div v-for="group in sensitivity" :key="group.chartTitle" class="rounded-lg border p-3">
            <p class="mb-2 text-xs font-medium text-muted-foreground">{{ group.chartTitle }}（适应度）</p>
            <VChart
              class="w-full shrink-0"
              :style="{ height: `${SENSITIVITY_CHART_HEIGHT}px` }"
              :option="sensitivityOption(group)"
              autoresize
            />
          </div>
        </div>
      </section>

      <Separator />

      <!-- 8760h 电量平衡曲线 -->
      <section>
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium">
          <AreaChart class="size-4 text-sky-600" />
          电量平衡曲线（全年 8760h）
        </h3>
        <div class="rounded-lg border p-3">
          <div class="no-capture mb-2 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
            <p class="text-xs font-medium text-muted-foreground">
              全年逐时电量平衡（单位：kWh），当前显示第 {{ zoomStart }} ~ {{ zoomEnd }} 小时（可拖动滑块或滚轮缩放）
            </p>
            <Button variant="ghost" size="sm" class="h-7 shrink-0 self-start px-2 text-xs" @click="resetZoom">
              重置
            </Button>
          </div>
          <VChart
            class="w-full shrink-0"
            :style="{ height: `${BALANCE_CHART_HEIGHT}px` }"
            :option="balanceOption"
            autoresize
            @datazoom="onBalanceZoom"
          />
        </div>
        <p class="mt-2 text-xs text-muted-foreground">
          导出报告包含：输入数据（24 项）、输入曲线（全年 8760h 负荷 / 风光标幺值 / 分时电价）、输出数据（最优配置、全年电量指标、投资与运行成本构成）、敏感性分析（三组 ±25% 变动）、逐时电量平衡（全年 8760h）五部分；当前逐时数据取自曲线模板算例，接入真实计算后由后端结果替换。
        </p>
      </section>
      </template>

      <!-- 空态：计算完成前不展示结果 -->
      <div
        v-else
        class="flex items-center justify-center gap-2 rounded-lg border border-dashed py-10 text-sm text-muted-foreground"
      >
        <FileSpreadsheet class="size-4" />
        完成计算后显示详细结果
      </div>
    </CardContent>
  </Card>

  <!-- 结果预览弹窗（图片形式） -->
  <Dialog v-model:open="previewOpen">
    <DialogContent class="max-w-5xl">
      <DialogHeader>
        <DialogTitle>结果预览</DialogTitle>
        <DialogDescription>以下为计算结果区域截图，以图片形式预览</DialogDescription>
      </DialogHeader>
      <div class="max-h-[70vh] overflow-auto rounded-lg border">
        <img v-if="previewImage" :src="previewImage" alt="计算结果预览" class="w-full" />
      </div>
      <DialogFooter>
        <Button variant="outline" @click="previewOpen = false">关闭</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
