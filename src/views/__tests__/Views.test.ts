import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import { createRouter, createMemoryHistory } from "vue-router";
import HomeView from "@/views/HomeView.vue";

const router = createRouter({
  history: createMemoryHistory(),
  routes: [{ path: "/", component: HomeView }],
});

describe("HomeView 主页面（无登录，直接进入）", () => {
  it("渲染数据上传、参数、计算、结果各区", async () => {
    router.push("/");
    await router.isReady();
    const wrapper = mount(HomeView, { global: { plugins: [router] } });
    const text = wrapper.text();
    expect(text).toContain("开始");
    expect(text).toContain("参数输入与确认");
    expect(text).toContain("计算历史");
    expect(text).toContain("计算结果");
    expect(text).toContain("上传输入文件");
    expect(text).toContain("上传发电及负荷曲线");
  });

  it("不含登录相关文案（登录/密码/验证码/退出登录）", async () => {
    router.push("/");
    await router.isReady();
    const wrapper = mount(HomeView, { global: { plugins: [router] } });
    const text = wrapper.text();
    expect(text).not.toContain("退出登录");
    expect(text).not.toContain("验证码");
  });
});
