import { describe, expect, it } from "vitest";
import { transportAssistantDeltaToMessageEvent } from "./chat-message-transport-adapter";

describe("transport message adapter", () => {
  it("maps stream payload to canonical assistant delta event", () => {
    const streamCache = {
      activationId: "activation-1",
      requestId: "request-1",
      updatedAt: "2026-07-26T08:00:00Z",
      persistedAssistantMessageId: "assistant-1",
      assistantText: "正文",
      toolStatusText: "正在执行",
      toolStatusState: "running",
      streamBlocks: [{ text: "正文", tools: [] }],
    };
    const transportEvent = transportAssistantDeltaToMessageEvent({
      kind: "tool_status",
      message: "正在执行",
      toolStatus: "running",
      streamCache,
    }, "conversation-1", "assistant-1");

    expect(transportEvent).toEqual({
      type: "assistant_delta",
      conversationId: "conversation-1",
      event: {
        assistantMessageId: "assistant-1",
        activationId: "activation-1",
        requestId: "request-1",
        revision: "2026-07-26T08:00:00Z",
        kind: "tool_status",
        delta: undefined,
        message: "正在执行",
        toolStatus: "running",
        streamCache: {
          activationId: "activation-1",
          requestId: "request-1",
          updatedAt: "2026-07-26T08:00:00Z",
          assistantText: "正文",
          toolStatusText: "正在执行",
          toolStatusState: "running",
          streamBlocks: [{ text: "正文", tools: [] }],
          persistedAssistantMessageId: "assistant-1",
        },
      },
    });
  });
});
