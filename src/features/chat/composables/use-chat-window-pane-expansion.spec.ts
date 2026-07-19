import { describe, expect, it } from "vitest";
import {
  cssWidthToPhysical,
  normalizeExternalPaneCssWidth,
} from "./use-chat-window-pane-expansion";

describe("chat window pane expansion sizing", () => {
  it("uses the measured webview ratio instead of the fallback DPI ratio", () => {
    expect(cssWidthToPhysical(320, 800, 1000, 2)).toBe(400);
  });

  it("falls back to devicePixelRatio when the viewport cannot be measured", () => {
    expect(cssWidthToPhysical(320, 0, 0, 1.5)).toBe(480);
  });

  it("uses the same left pane width limits as the rendered layout", () => {
    expect(normalizeExternalPaneCssWidth("left", 100)).toBe(200);
    expect(normalizeExternalPaneCssWidth("left", 500)).toBe(360);
  });
});
