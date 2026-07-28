import { describe, expect, it } from "vitest";
import { apiConfigDisplayName, formatApiConfigOptionLabel } from "./api-config-display";

describe("formatApiConfigOptionLabel", () => {
  it("应以 reasoningEffort 实时补全历史 name 缺失的思维等级，并使用中点分隔", () => {
    expect(formatApiConfigOptionLabel({
      name: "cpa/gpt-5.5",
      model: "gpt-5.5",
      reasoningEffort: "medium",
    })).toBe("cpa · gpt-5.5 · 中");
  });

  it("应仅在聊天显示时把供应商和模型的斜杠改为中点", () => {
    const name = apiConfigDisplayName("惹", "gpt-5.6-terra", "high");
    expect(name).toBe("惹/gpt-5.6-terra · 高");
    expect(formatApiConfigOptionLabel({
      name,
      model: "gpt-5.6-terra",
      reasoningEffort: "high",
    })).toBe("惹 · gpt-5.6-terra · 高");
  });

  it("紧凑显示时供应商最多保留两个字符", () => {
    expect(formatApiConfigOptionLabel({
      name: "超长供应商/gpt-5.6-terra · 高",
      model: "gpt-5.6-terra",
      reasoningEffort: "high",
    }, undefined, { providerMaxCharacters: 2 })).toBe("超长 · gpt-5.6-terra · 高");
  });
});
