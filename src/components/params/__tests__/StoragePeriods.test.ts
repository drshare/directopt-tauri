import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import StoragePeriods from "@/components/params/StoragePeriods.vue";

describe("StoragePeriods 储能时段勾选", () => {
  it("渲染 24 个充电优先时段 + 24 个放电时段", () => {
    const wrapper = mount(StoragePeriods);
    expect(wrapper.text()).toContain("储能充电优先时段");
    expect(wrapper.text()).toContain("储能允许放电时段");
    // reka-ui Checkbox 渲染为 button[role=checkbox]，每列 24 个
    const checkboxes = wrapper.findAll('button[role="checkbox"]');
    expect(checkboxes.length).toBe(48);
    // 首尾时段文案
    expect(wrapper.text()).toContain("00:00-01:00");
    expect(wrapper.text()).toContain("23:00-24:00");
  });

  it("勾选/取消勾选时段正确更新已选数量", async () => {
    const wrapper = mount(StoragePeriods);
    expect(wrapper.text()).toContain("已选 0 个时段");
    const boxes = wrapper.findAll('button[role="checkbox"]');
    // 勾选充电优先的第一个时段
    await boxes[0].trigger("click");
    expect(wrapper.text()).toContain("已选 1 个时段");
    // 再勾选充电优先的第二个时段
    await boxes[1].trigger("click");
    expect(wrapper.text()).toContain("已选 2 个时段");
    // 取消第一个
    await boxes[0].trigger("click");
    expect(wrapper.text()).toContain("已选 1 个时段");
    expect(wrapper.findAll('button[aria-checked="true"]').length).toBe(1);
  });
});
