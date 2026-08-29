/**
 * SSR 运行时冒烟测试
 * 用 Vue SSR 实际渲染页面组件，捕获运行时错误（script setup、render、图标、组件 API）
 * 本项目无需登录系统，直接渲染主页面。
 * 运行：node scripts/ssr-smoke.mjs
 */
import { createServer } from "vite";
import { createSSRApp } from "vue";
import { renderToString } from "@vue/server-renderer";
import { createRouter, createMemoryHistory } from "vue-router";

const server = await createServer({
  server: { middlewareMode: true },
  appType: "custom",
  logLevel: "error",
});

const errors = [];
const warnings = [];

const HomeView = (await server.ssrLoadModule("/src/views/HomeView.vue")).default;

const router = createRouter({
  history: createMemoryHistory(),
  routes: [{ path: "/", component: HomeView }],
});

async function render(name, comp) {
  try {
    const app = createSSRApp(comp);
    app.use(router);
    // 捕获 Vue 警告
    const origWarn = console.warn;
    const collected = [];
    console.warn = (...a) => collected.push(a.join(" "));
    const html = await renderToString(app);
    console.warn = origWarn;
    const ok = html && html.length > 100;
    console.log(`[SSR] ${name}: ${ok ? "OK" : "SHORT-EMPTY"} (html ${html.length} chars)`);
    if (!ok) errors.push(`${name}: html too short`);
    if (collected.length) {
      console.log(`      ${collected.length} warning(s):`);
      collected.slice(0, 8).forEach((w) => console.log("      ⚠ " + w.slice(0, 200)));
      warnings.push(...collected);
    }
  } catch (e) {
    errors.push(`${name}: ${e?.message || e}`);
    console.error(`[SSR] ${name}: FAILED -> ${e?.stack?.split("\n").slice(0, 6).join("\n      ")}`);
  }
}

await render("HomeView", HomeView);
await server.close();

console.log("\n===== 结果 =====");
if (errors.length) {
  console.error(`❌ ${errors.length} 个错误`);
  errors.forEach((e) => console.error("   - " + e));
  process.exit(1);
}
if (warnings.length) {
  console.log(`⚠ ${warnings.length} 个警告（已列出，需人工判断）`);
} else {
  console.log("✅ 无错误、无警告");
}
