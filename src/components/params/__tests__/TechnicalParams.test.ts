import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import TechnicalParams from "@/components/params/TechnicalParams.vue";

describe("TechnicalParams 技术参数", () => {
  it("渲染全部 12 个字段与默认值", () => {
    const wrapper = mount(TechnicalParams);
    const text = wrapper.text();
    expect(text).toContain("储能充放电深度");
    expect(text).toContain("电池充放电倍率");
    expect(text).toContain("储能初始电量");
    expect(text).toContain("储能充电效率");
    expect(text).toContain("储能放电效率");
    expect(text).toContain("接入公共电网容量");
    expect(text).toContain("平均负荷率");
    expect(text).toContain("自发自用占总可用发电量比例下限");
    expect(text).toContain("自发自用占总用电量比例下限");
    expect(text).toContain("余电上网比例上限");
    expect(text).toContain("余电最大上网功率");
    expect(text).toContain("弃电率上限");
    // 默认值抽查
    const inputs = wrapper.findAll("input").map((i) => (i.element as HTMLInputElement).value);
    expect(inputs).toContain("85"); // DOD
    expect(inputs).toContain("93"); // 充电效率
    expect(inputs).toContain("92"); // 放电效率
    expect(inputs).toContain("60"); // 自发自用下限
    expect(inputs).toContain("20"); // 弃电率上限
  });
});
