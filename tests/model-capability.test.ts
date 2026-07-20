import { describe, expect, it } from "vitest";

import {
  buildModelCapability,
  buildReasoningCapability,
  FALLBACK_REASONING_EFFORT_OPTIONS,
} from "../src/features/config/utils/model-capability";

describe("model capability reasoning abstraction", () => {
  it("treats reasoning false as unsupported", () => {
    expect(buildReasoningCapability({ reasoning: false, reasoningOptions: [{ type: "effort", values: ["low", "high"] }] })).toEqual({
      supportsReasoning: false,
      reasoningEffortOptions: [],
    });
  });

  it("adds default and extracts effort options from raw reasoning options", () => {
    expect(buildReasoningCapability({ reasoning: true, reasoningOptions: [{ type: "effort", values: ["Low", "high", "high"] }] })).toEqual({
      supportsReasoning: true,
      reasoningEffortOptions: ["default", "low", "high"],
    });
  });

  it("keeps reasoning supported and exposes only default when provider only exposes toggle", () => {
    expect(buildModelCapability({
      reasoning: true,
      reasoningOptions: [{ type: "toggle" }],
      enableTools: true,
    })).toEqual({
      contextWindowMax: undefined,
      maxOutputTokensMax: undefined,
      enableImage: undefined,
      enableVideo: undefined,
      enableAudio: undefined,
      enableTools: true,
      documentationUrl: undefined,
      reasoning: {
        supportsReasoning: true,
        reasoningEffortOptions: ["default"],
      },
    });
  });

  it("uses default plus full fallback union on cache miss", () => {
    expect(buildModelCapability({
      metadataFound: false,
    }).reasoning).toEqual({
      supportsReasoning: true,
      reasoningEffortOptions: ["default", ...FALLBACK_REASONING_EFFORT_OPTIONS],
    });
  });

  it("keeps documentation url on model capability snapshot", () => {
    expect(buildModelCapability({
      reasoning: true,
      reasoningEffortOptions: ["medium"],
      documentationUrl: "https://api-docs.deepseek.com/",
    }).documentationUrl).toBe("https://api-docs.deepseek.com/");
  });
});
