import { describe, expect, it } from "vitest";
import { normalizeChatMonitorPanelMode, normalizeChatRightPanelMode } from "./chat-ui-layout-storage";

describe("normalizeChatRightPanelMode", () => {
  it("preserves top-level panel modes", () => {
    expect(normalizeChatRightPanelMode("sideChat")).toBe("sideChat");
    expect(normalizeChatRightPanelMode("monitor")).toBe("monitor");
  });

  it("migrates legacy monitor tabs to the monitor panel", () => {
    expect(normalizeChatRightPanelMode("delegate")).toBe("monitor");
    expect(normalizeChatRightPanelMode("review")).toBe("monitor");
  });
});

describe("normalizeChatMonitorPanelMode", () => {
  it("keeps monitor tabs independent from the top-level panel", () => {
    expect(normalizeChatMonitorPanelMode("tasks")).toBe("tasks");
    expect(normalizeChatMonitorPanelMode("review")).toBe("tools");
  });
});
