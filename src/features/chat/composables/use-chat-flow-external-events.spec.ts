import { describe, expect, it } from "vitest";
import {
  externalTerminalTargetsRound,
  shouldSuppressStoppedHistoryActivation,
} from "./use-chat-flow-external-events";

describe("external chat terminal identity", () => {
  it("rejects a terminal from another activation", () => {
    expect(externalTerminalTargetsRound(
      { phase: "streaming", gen: 2, messageId: "assistant-new" },
      "activation-new",
      { activationId: "activation-old" },
    )).toBe(false);
  });

  it("rejects a formal completion for another assistant message", () => {
    expect(externalTerminalTargetsRound(
      { phase: "streaming", gen: 2, messageId: "assistant-new" },
      "",
      { assistantMessageId: "assistant-old" },
    )).toBe(false);
  });

  it("keeps legacy terminal payloads without identity usable", () => {
    expect(externalTerminalTargetsRound(
      { phase: "queued", gen: 1, messageId: "assistant-1" },
      "activation-1",
      {},
    )).toBe(true);
  });

  it("停止后仍接收正式 historyFlushed，只抑制其旧轮次激活投影", () => {
    expect(shouldSuppressStoppedHistoryActivation(true, true)).toBe(true);
    expect(shouldSuppressStoppedHistoryActivation(false, true)).toBe(false);
    expect(shouldSuppressStoppedHistoryActivation(true, false)).toBe(false);
  });
});
