import { describe, expect, it } from "vitest";
import { normalizeResponseStrategy } from "./helpers";

describe("normalizeResponseStrategy", () => {
  it("将缺失或未知策略收敛为智能判断", () => {
    expect(normalizeResponseStrategy()).toBe("smart_judge");
    expect(normalizeResponseStrategy("unknown")).toBe("smart_judge");
  });

  it("保留明确选择的始终回复", () => {
    expect(normalizeResponseStrategy("always_reply")).toBe("always_reply");
  });
});
