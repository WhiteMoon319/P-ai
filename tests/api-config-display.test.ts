import { describe, expect, it } from "vitest";

import { sortReasoningEffortValues } from "../src/features/config/utils/api-config-display";

describe("sortReasoningEffortValues", () => {
  it("keeps legal efforts in fixed order regardless of input order or checked-first bias", () => {
    expect(sortReasoningEffortValues(["high", "default", "max", "low", "medium", "xhigh", "none", "minimal"])).toEqual([
      "default",
      "none",
      "minimal",
      "low",
      "medium",
      "high",
      "xhigh",
      "max",
    ]);
  });

  it("does not move selected efforts ahead of capability order", () => {
    // 模拟：已勾选 high/max，能力列表为固定全集
    expect(sortReasoningEffortValues([
      "high",
      "max",
      "default",
      "none",
      "minimal",
      "low",
      "medium",
      "xhigh",
    ])).toEqual([
      "default",
      "none",
      "minimal",
      "low",
      "medium",
      "high",
      "xhigh",
      "max",
    ]);
  });

  it("places unknown efforts after legal ones and keeps relative order by locale", () => {
    expect(sortReasoningEffortValues(["legacy-b", "high", "legacy-a", "default"])).toEqual([
      "default",
      "high",
      "legacy-a",
      "legacy-b",
    ]);
  });
});
