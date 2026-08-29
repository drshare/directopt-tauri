<script setup lang="ts">
import { ref, watch } from "vue";
import { AlertTriangle, FileSpreadsheet, LoaderCircle } from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { SheetPreview, TemplateMeta } from "@/lib/templates";
import { loadTemplatePreview } from "@/lib/templates";

const props = defineProps<{
  open: boolean;
  template: TemplateMeta | null;
}>();

const emit = defineEmits<{
  (e: "update:open", value: boolean): void;
}>();

const loading = ref(false);
const error = ref("");
const sheets = ref<SheetPreview[]>([]);
const activeSheet = ref("");

watch(
  () => [props.open, props.template?.name] as const,
  async ([open, name]) => {
    if (!open || !name) return;
    loading.value = true;
    error.value = "";
    sheets.value = [];
    try {
      const preview = await loadTemplatePreview(props.template!);
      sheets.value = preview.sheets;
      activeSheet.value = preview.sheets[0]?.name ?? "";
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  },
  { immediate: true },
);

function onOpenChange(value: boolean) {
  emit("update:open", value);
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="sm:max-w-4xl">
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2">
          <FileSpreadsheet class="size-5 text-emerald-600" />
          模板预览：{{ template?.name }}
        </DialogTitle>
        <DialogDescription>{{ template?.description }}</DialogDescription>
      </DialogHeader>

      <div v-if="loading" class="flex min-h-40 items-center justify-center gap-2 text-sm text-muted-foreground">
        <LoaderCircle class="size-4 animate-spin" />
        正在解析模板文件…
      </div>

      <div v-else-if="error" class="flex min-h-40 items-center justify-center gap-2 text-sm text-destructive">
        <AlertTriangle class="size-4" />
        {{ error }}
      </div>

      <Tabs v-else v-model="activeSheet" class="min-h-0 gap-2">
        <TabsList>
          <TabsTrigger v-for="sheet in sheets" :key="sheet.name" :value="sheet.name">
            {{ sheet.name }}
            <Badge variant="secondary" class="ml-1.5">{{ sheet.totalRows }} 行</Badge>
          </TabsTrigger>
        </TabsList>

        <TabsContent
          v-for="sheet in sheets"
          :key="sheet.name"
          :value="sheet.name"
          class="min-h-0"
        >
          <div class="max-h-[50dvh] overflow-auto rounded-md border">
            <Table class="min-w-max">
              <TableHeader class="sticky top-0 z-10 bg-muted">
                <TableRow>
                  <TableHead class="w-12 text-center">#</TableHead>
                  <TableHead v-for="(cell, i) in sheet.rows[0] ?? []" :key="i">
                    {{ cell }}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                <TableRow
                  v-for="(row, r) in sheet.rows.slice(1)"
                  :key="r"
                  class="odd:bg-muted/40"
                >
                  <TableCell class="text-center text-xs text-muted-foreground">
                    {{ r + 2 }}
                  </TableCell>
                  <TableCell v-for="(cell, c) in row" :key="c" class="whitespace-nowrap">
                    {{ cell }}
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </div>
          <p v-if="sheet.truncated" class="mt-2 text-xs text-muted-foreground">
            仅展示前 {{ sheet.rows.length }} 行，完整内容共 {{ sheet.totalRows }} 行，请下载模板查看。
          </p>
        </TabsContent>
      </Tabs>
    </DialogContent>
  </Dialog>
</template>
