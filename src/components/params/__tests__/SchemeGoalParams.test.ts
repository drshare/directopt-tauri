import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import SchemeGoalParams from "@/components/params/SchemeGoalParams.vue";

describe("SchemeGoalParams 方案与目标", () => {
  it("渲染输配电费方案一/二 与 3 个优化目标", () => {
    const wrapper = mount(SchemeGoalParams);
    expect(wrapper.text()).toContain("输配电费缴纳方案");
    expect(wrapper.text()).toContain("方案一");
    expect(wrapper.text()).toContain("方案二");
    expect(wrapper.text()).toContain("综合电价最低");
    expect(wrapper.text()).toContain("绿电电价最低");
    expect(wrapper.text()).toContain("初投资最低");
    // reka-ui RadioGroup 渲染为 button[role=radio]；5 个选项，默认选中 2 个
    const radios = wrapper.findAll('button[role="radio"]');
    expect(radios.length).toBe(5);
    expect(wrapper.findAll('button[aria-checked="true"]').length).toBe(2);
  });

  it("切换方案与优化目标", async () => {
    const wrapper = mount(SchemeGoalParams);
    const radios = wrapper.findAll('button[role="radio"]');
    // radios 顺序：方案一, 方案二, 综合电价, 绿电电价, 初投资
    await radios[1].trigger("click"); // 方案二
    await radios[3].trigger("click"); // 绿电电价最低
    expect(radios[0].attributes("aria-checked")).toBe("false");
    expect(radios[1].attributes("aria-checked")).toBe("true"); // 方案二选中
    expect(radios[2].attributes("aria-checked")).toBe("false");
    expect(radios[3].attributes("aria-checked")).toBe("true"); // 绿电电价最低选中
    expect(radios[4].attributes("aria-checked")).toBe("false");
  });
});
