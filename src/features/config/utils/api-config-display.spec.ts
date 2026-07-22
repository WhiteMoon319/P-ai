import { describe, expect, it } from "vitest";
import { formatApiConfigOptionLabel } from "./api-config-display";

describe("formatApiConfigOptionLabel", () => {
  it("应以 reasoningEffort 实时补全历史 name 缺失的思维等级", () => {
    expect(formatApiConfigOptionLabel({
      name: "cpa/gpt-5.5",
      model: "gpt-5.5",
      reasoningEffort: "medium",
    })).toBe("cpa/gpt-5.5 · 中");
  });
});
