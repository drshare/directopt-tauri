import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "@/views/HomeView.vue";

/**
 * 路由配置
 * - 本项目为桌面端工具，无需登录系统，启动后直接进入主界面
 * - 使用 hash 模式，便于 Tauri 桌面容器与静态部署（无需服务端重写规则）
 */
export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: HomeView,
      meta: { title: "参数输入与计算 · 绿电直连新能源优化配置软件 V2.2" },
    },
  ],
});

router.afterEach((to) => {
  const title = (to.meta?.title as string) ?? "绿电直连新能源优化配置软件";
  document.title = title;
});

export default router;
