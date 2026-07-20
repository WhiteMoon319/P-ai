import { describe, expect, it } from "vitest";
import type { ApiConfigItem } from "../../../types/app";
import { buildApiConfigSelectionTree } from "./api-config-selection-tree";

function apiConfig(overrides: Partial<ApiConfigItem>): ApiConfigItem {
  return {
    id: "provider-a::model-default",
    name: "Provider A/gpt-5.6-terra · 默认",
    requestFormat: "openai",
    enableText: true,
    enableImage: false,
    enableAudio: false,
    enableVideo: false,
    enableTools: true,
    tools: [],
    baseUrl: "https://example.com/v1",
    apiKey: "",
    model: "gpt-5.6-terra",
    reasoningEffort: "default",
    temperature: 1,
    customTemperatureEnabled: false,
    contextWindowTokens: 128_000,
    customMaxOutputTokensEnabled: false,
    maxOutputTokens: 4_096,
    ...overrides,
  };
}

describe("buildApiConfigSelectionTree", () => {
  it("按渠道、同公共配置模型和思维等级分组，并保留原 apiConfigId", () => {
    const tree = buildApiConfigSelectionTree([
      apiConfig({ id: "provider-a::model-default", reasoningEffort: "default" }),
      apiConfig({ id: "provider-a::model-medium", reasoningEffort: "medium" }),
      apiConfig({
        id: "provider-a::model-max",
        reasoningEffort: "max",
        contextWindowTokens: 256_000,
      }),
    ]);

    expect(tree).toHaveLength(1);
    expect(tree[0].name).toBe("Provider A");
    expect(tree[0].models).toHaveLength(2);
    expect(tree[0].models[0].leaves.map((leaf) => leaf.id)).toEqual([
      "provider-a::model-default",
      "provider-a::model-medium",
    ]);
    expect(tree[0].models[0].summaryFields).toContain("contextWindowTokens");
    expect(tree[0].models[1].leaves[0].id).toBe("provider-a::model-max");
  });

  it("未知思维等级仍作为可选叶子保留", () => {
    const tree = buildApiConfigSelectionTree([
      apiConfig({ id: "provider-a::model-legacy", reasoningEffort: "legacy" }),
    ]);

    expect(tree[0].models[0].leaves).toEqual([
      expect.objectContaining({ id: "provider-a::model-legacy", label: "legacy" }),
    ]);
  });
});
