import { describe, expect, it } from "vitest";
import { reconcileAuthoritativeConversationMessage } from "./chat-message-reconciliation";

function message(text: string, providerMeta?: Record<string, unknown>) {
  return {
    id: "assistant-1",
    role: "assistant",
    parts: [{ type: "text", text }],
    providerMeta,
  };
}

describe("reconcileAuthoritativeConversationMessage", () => {
  it("正式消息替换同 ID 的流式投影并保留稳定渲染 ID", () => {
    const result = reconcileAuthoritativeConversationMessage(
      message("partial", { _streaming: true, _stableRenderId: "stable-1" }),
      message("final"),
    );

    expect(result.parts[0].text).toBe("final");
    expect(result.providerMeta?._stableRenderId).toBe("stable-1");
  });

  it("本地已经冻结的正式正文不被晚到消息改写，只接收权威用量", () => {
    const result = reconcileAuthoritativeConversationMessage(
      message("stopped", { contextUsagePercent: 10 }),
      message("later", { contextUsagePercent: 25 }),
    );

    expect(result.parts[0].text).toBe("stopped");
    expect(result.providerMeta?.contextUsagePercent).toBe(25);
  });

  it("冻结正文仍可接收权威 planCard 元数据", () => {
    const result = reconcileAuthoritativeConversationMessage(
      message("frozen"),
      message("later", {
        planCard: { action: "present", path: ".pai/plan/example.md" },
      }),
    );

    expect(result.parts[0].text).toBe("frozen");
    expect(result.providerMeta?.planCard).toEqual({
      action: "present",
      path: ".pai/plan/example.md",
    });
  });

  it("目标消息终态刷新可以强制用正式正文收口", () => {
    const result = reconcileAuthoritativeConversationMessage(
      message("partial"),
      message("final"),
      { forceReplace: true },
    );

    expect(result.parts[0].text).toBe("final");
  });
});
