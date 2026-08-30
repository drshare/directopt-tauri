<script setup lang="ts">
import { computed } from "vue";
import { BookOpenText } from "@lucide/vue";
import { renderMarkdown } from "@/lib/markdown";
// 直接引入项目文档原文（Vite ?raw），文档更新后界面自动同步
import docRaw from "../../../docs/本项目计算引擎详细计算过程.md?raw";

const html = computed(() => renderMarkdown(docRaw));
</script>

<template>
  <div class="calc-doc flex h-full min-h-0 flex-col overflow-hidden rounded-lg border bg-background">
    <div
      class="flex shrink-0 items-center gap-2 border-b bg-muted/40 px-3 py-2 text-xs font-medium text-muted-foreground"
    >
      <BookOpenText class="size-4 shrink-0 text-teal-600" />
      <span>计算过程文档 ·《本项目计算引擎详细计算过程》</span>
      <span class="ml-auto shrink-0 text-[10px] font-normal">docs/本项目计算引擎详细计算过程.md</span>
    </div>
    <!-- 文档为项目内可信静态内容 -->
    <!-- eslint-disable-next-line vue/no-v-html -->
    <div class="md-body min-h-0 flex-1 overflow-y-auto px-4 py-3 text-xs leading-relaxed" v-html="html"></div>
  </div>
</template>

<style>
/* v-html 内容不受 Vue scoped 作用，使用 .calc-doc 前缀防止样式泄漏 */
.calc-doc .md-body h1 {
  font-size: 1.05rem;
  font-weight: 700;
  margin: 0.2rem 0 0.75rem;
}
.calc-doc .md-body h2 {
  font-size: 0.95rem;
  font-weight: 700;
  margin: 1.1rem 0 0.5rem;
  padding-bottom: 0.3rem;
  border-bottom: 1px solid rgb(128 128 128 / 0.25);
}
.calc-doc .md-body h3 {
  font-size: 0.85rem;
  font-weight: 600;
  margin: 0.9rem 0 0.4rem;
}
.calc-doc .md-body h4 {
  font-size: 0.8rem;
  font-weight: 600;
  margin: 0.7rem 0 0.3rem;
}
.calc-doc .md-body p {
  margin: 0.4rem 0;
}
.calc-doc .md-body ul {
  list-style: disc;
  padding-left: 1.2rem;
  margin: 0.4rem 0;
}
.calc-doc .md-body ol {
  list-style: decimal;
  padding-left: 1.4rem;
  margin: 0.4rem 0;
}
.calc-doc .md-body li {
  margin: 0.15rem 0;
}
.calc-doc .md-body table {
  width: 100%;
  border-collapse: collapse;
  margin: 0.5rem 0;
  font-size: 11px;
  display: block;
  overflow-x: auto;
}
.calc-doc .md-body th,
.calc-doc .md-body td {
  border: 1px solid rgb(128 128 128 / 0.3);
  padding: 3px 7px;
  text-align: left;
  vertical-align: top;
  white-space: nowrap;
}
.calc-doc .md-body th {
  background: rgb(128 128 128 / 0.1);
  font-weight: 600;
}
.calc-doc .md-body code {
  background: rgb(128 128 128 / 0.15);
  border-radius: 3px;
  padding: 1px 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
}
.calc-doc .md-body pre {
  background: rgb(128 128 128 / 0.12);
  border: 1px solid rgb(128 128 128 / 0.25);
  border-radius: 6px;
  padding: 8px;
  overflow-x: auto;
  margin: 0.5rem 0;
}
.calc-doc .md-body pre code {
  background: transparent;
  padding: 0;
}
.calc-doc .md-body blockquote {
  border-left: 3px solid rgb(128 128 128 / 0.4);
  padding-left: 0.6rem;
  margin: 0.5rem 0;
  opacity: 0.85;
}
.calc-doc .md-body hr {
  border: none;
  border-top: 1px solid rgb(128 128 128 / 0.25);
  margin: 0.9rem 0;
}
.calc-doc .md-body strong {
  font-weight: 700;
}
</style>
