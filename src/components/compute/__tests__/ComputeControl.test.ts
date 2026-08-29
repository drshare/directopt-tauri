import { mount } from "@vue/test-utils";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ComputeControl from "@/components/compute/ComputeControl.vue";
import { computation } from "@/composables/useComputation";
import { params } from "@/composables/useParams";

describe("ComputeControl 计算状态机（含参数校验 FR-5）", () => {
  beforeEach(() => {
    computation.status = "idle";
    computation.progress = 0;
    computation.queueCount = 0;
    // 为通过校验，设置一个合法的曲线文件与合理参数
    params.curveFile = "curvetemplate_ldzl_2.1.xlsx";
    params.dod = "80";
    params.initialSoc = "20";
    params.crossoverRate = "0.5";
    params.mutationRate = "0.3";
  });

  it("初始为 idle（未开始）", () => {
    const wrapper = mount(ComputeControl);
    expect(wrapper.text()).toContain("未开始");
    expect(computation.status).toBe("idle");
  });

  it("曲线未上传时校验失败并阻止计算", async () => {
    params.curveFile = "";
    const wrapper = mount(ComputeControl);
    await wrapper.find("button").trigger("click");
    expect(computation.status).toBe("error");
    expect(wrapper.text()).toContain("参数校验未通过");
    expect(wrapper.text()).toContain("发电及负荷曲线为必选");
  });

  it("DOD+初始电量 <100 校验失败", async () => {
    params.dod = "70";
    params.initialSoc = "10";
    const wrapper = mount(ComputeControl);
    await wrapper.find("button").trigger("click");
    expect(computation.status).toBe("error");
    expect(wrapper.text()).toContain("储能初始电量 + 充放电深度 ≥ 100%");
  });

  it("校验通过：排队 → 计算中(进度) → 完成", async () => {
    vi.useFakeTimers();
    const wrapper = mount(ComputeControl);
    await wrapper.find("button").trigger("click");

    // 排队
    expect(computation.status).toBe("queued");
    expect(wrapper.text()).toContain("任务排队中");
    await vi.advanceTimersByTimeAsync(1600);

    // 计算中
    expect(computation.status).toBe("running");
    expect(wrapper.text()).toContain("计算中");
    expect(wrapper.text()).toContain("请勿关闭页面");
    await vi.advanceTimersByTimeAsync(1500);

    // 完成
    expect(computation.status).toBe("done");
    expect(wrapper.text()).toContain("计算完成");
    vi.useRealTimers();
  });

  it("计算中再次点击被禁用", async () => {
    vi.useFakeTimers();
    const wrapper = mount(ComputeControl);
    await wrapper.find("button").trigger("click");
    expect(computation.status).toBe("queued");
    const btn = wrapper.find("button");
    expect((btn.element as HTMLButtonElement).disabled).toBe(true);
    vi.clearAllTimers();
    vi.useRealTimers();
  });
});
