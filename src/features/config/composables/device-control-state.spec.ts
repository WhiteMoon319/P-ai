import { describe, expect, it } from "vitest";
import {
  deviceControlStateBadgeClass,
  deviceControlStateLabelKey,
  deviceControlStatusLines,
  type DeviceControlStatus,
} from "./device-control-state";

function buildStatus(overrides: Partial<DeviceControlStatus> = {}): DeviceControlStatus {
  return {
    shizukuAvailable: false,
    shizukuGranted: false,
    rootAvailable: false,
    privilegeState: "disabled",
    ...overrides,
  };
}

// t 注入桩：直接拼 key + 插值参数
const stubT = (key: string, opts?: Record<string, unknown>) => {
  const optsText = opts ? JSON.stringify(opts) : "";
  return `${key}${optsText}`;
};

describe("deviceControlStateLabelKey", () => {
  it("各提权状态映射到对应 i18n key", () => {
    expect(deviceControlStateLabelKey("disabled")).toBe("config.tools.deviceControlStateDisabled");
    expect(deviceControlStateLabelKey("shizuku_pending")).toBe("config.tools.deviceControlStateShizukuPending");
    expect(deviceControlStateLabelKey("shizuku_ready")).toBe("config.tools.deviceControlStateShizukuReady");
    expect(deviceControlStateLabelKey("root_ready")).toBe("config.tools.deviceControlStateRootReady");
  });
});

describe("deviceControlStateBadgeClass", () => {
  it("就绪状态用 success 徽章", () => {
    expect(deviceControlStateBadgeClass("shizuku_ready")).toBe("badge-success");
    expect(deviceControlStateBadgeClass("root_ready")).toBe("badge-success");
  });

  it("待授权用 info 徽章，禁用用 warning 徽章", () => {
    expect(deviceControlStateBadgeClass("shizuku_pending")).toBe("badge-info");
    expect(deviceControlStateBadgeClass("disabled")).toBe("badge-warning");
  });
});

describe("deviceControlStatusLines", () => {
  it("生成 Shizuku/授权/root 三行详情", () => {
    const lines = deviceControlStatusLines(
      buildStatus({ shizukuAvailable: true, shizukuGranted: true, rootAvailable: false }),
      stubT,
    );
    expect(lines).toHaveLength(3);
    expect(lines[0]).toContain("config.tools.deviceControlDetailShizuku");
    expect(lines[0]).toContain("✓");
    expect(lines[1]).toContain("✓");
    expect(lines[2]).toContain("✗");
  });

  it("全部不可用时三行均为 ✗", () => {
    const lines = deviceControlStatusLines(buildStatus(), stubT);
    expect(lines.every((line) => line.includes("✗"))).toBe(true);
  });
});