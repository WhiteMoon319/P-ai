import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("../src/services/tauri-api", () => ({
  isTauriRuntimeAvailable: () => false,
}));

import {
  normalizeUiSizeScale,
  stepUiSizeScale,
  uiSizeTokensFor,
  useUiSizeAppearance,
  UI_SIZE_DEFAULT_SCALE,
  UI_SIZE_MAX_SCALE,
  UI_SIZE_MIN_SCALE,
} from "../src/features/shell/composables/use-ui-size-appearance";

describe("ui size appearance", () => {
  it("normalizes legacy presets and continuous values within the supported range", () => {
    expect(normalizeUiSizeScale("small")).toBe(75);
    expect(normalizeUiSizeScale("default")).toBe(100);
    expect(normalizeUiSizeScale("large")).toBe(125);
    expect(normalizeUiSizeScale("extraLarge")).toBe(150);
    expect(normalizeUiSizeScale(74)).toBe(75);
    expect(normalizeUiSizeScale(151)).toBe(150);
    expect(normalizeUiSizeScale(112.6)).toBe(113);
    expect(normalizeUiSizeScale(null)).toBe(100);
    expect(normalizeUiSizeScale(" ")).toBe(100);
    expect(normalizeUiSizeScale("invalid")).toBe(100);
  });

  it("derives every typography and control token from the selected scale", () => {
    expect(uiSizeTokensFor(75)).toMatchObject({
      textMicro: "6.75px",
      textCaption: "8.25px",
      textXs: "9px",
      textSm: "10.5px",
      textBase: "12px",
      text2Xl: "18px",
      sizeField: "3.12px",
      border: "0.75px",
    });
    expect(uiSizeTokensFor(125)).toMatchObject({
      textMicro: "11.25px",
      textCaption: "13.75px",
      textXs: "15px",
      textSm: "17.5px",
      textBase: "20px",
      text2Xl: "30px",
      markdownHeading1: "20.4px",
      markdownHeading2: "19.6px",
      markdownHeading3: "18.8px",
      markdownHeading4: "18px",
      markdownDocumentHeading1: "30px",
      markdownDocumentHeading2: "25.6px",
      markdownDocumentHeading3: "22.4px",
      markdownDocumentHeading4: "20.4px",
      sizeField: "5.2px",
      border: "1.25px",
    });
  });

  it("steps ui size by 10% and clamps within the supported range", () => {
    const { setUiSizeScale, uiSizeScale } = useUiSizeAppearance();
    setUiSizeScale(UI_SIZE_DEFAULT_SCALE);
    expect(stepUiSizeScale(1)).toBe(110);
    expect(uiSizeScale.value).toBe(110);
    expect(stepUiSizeScale(-1)).toBe(100);
    setUiSizeScale(UI_SIZE_MIN_SCALE);
    expect(stepUiSizeScale(-1)).toBe(UI_SIZE_MIN_SCALE);
    setUiSizeScale(UI_SIZE_MAX_SCALE);
    expect(stepUiSizeScale(1)).toBe(UI_SIZE_MAX_SCALE);
    setUiSizeScale(145);
    expect(stepUiSizeScale(1)).toBe(UI_SIZE_MAX_SCALE);
  });
});
