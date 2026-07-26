import { describe, expect, it } from "vitest";
import {
  sidebarAssistantDeltaToMessageEvent,
  sidebarRoundFinishedToMessageEvent,
  sidebarRoundStartedToMessageEvent,
} from "./sidebar-chat-message-adapter";

describe("sidebar chat message adapter", () => {
  it("normalizes round started payload", () => {
    expect(sidebarRoundStartedToMessageEvent({
      conversationId: "conversation-1",
      assistantMessageId: "assistant-1",
      activationId: "activation-1",
    }, "")).toMatchObject({
      type: "round_started",
      conversationId: "conversation-1",
      assistantMessageId: "assistant-1",
      activationId: "activation-1",
    });
  });

  it("unwraps WebSocket assistant delta and stream cache", () => {
    expect(sidebarAssistantDeltaToMessageEvent({
      conversationId: "conversation-1",
      event: {
        kind: "assistant_tool_event",
        activationId: "activation-top-level",
        requestId: "request-top-level",
        message: "tool",
        streamCache: {
          activationId: "activation-1",
          persistedAssistantMessageId: "assistant-1",
          updatedAt: "2026-07-26T08:00:00Z",
          streamBlocks: [{ text: "正文", tools: [] }],
        },
      },
    }, "")).toEqual({
      type: "assistant_delta",
      conversationId: "conversation-1",
      event: {
        assistantMessageId: "assistant-1",
        activationId: "activation-top-level",
        requestId: "request-top-level",
        revision: "2026-07-26T08:00:00Z",
        kind: "assistant_tool_event",
        delta: undefined,
        message: "tool",
        toolStatus: undefined,
        streamCache: {
          activationId: "activation-1",
          requestId: undefined,
          updatedAt: "2026-07-26T08:00:00Z",
          assistantText: undefined,
          toolStatusText: undefined,
          toolStatusState: undefined,
          streamBlocks: [{ text: "正文", tools: [] }],
          persistedAssistantMessageId: "assistant-1",
        },
      },
    });
  });

  it("maps failed roundFinished notification to round_failed", () => {
    expect(sidebarRoundFinishedToMessageEvent({
      conversationId: "conversation-1",
      status: "failed",
      error: "失败原因",
    }, "", "assistant-1")).toMatchObject({
      failed: true,
      event: {
        type: "round_failed",
        conversationId: "conversation-1",
        assistantMessageId: "assistant-1",
        error: "失败原因",
      },
    });
  });

  it("uses a terminal payload message id before falling back to the tracked id", () => {
    expect(sidebarRoundFinishedToMessageEvent({
      conversationId: "conversation-1",
      status: "completed",
      assistantMessageId: "assistant-formal",
      assistantMessage: {
        id: "assistant-formal",
        role: "assistant",
        parts: [{ type: "text", text: "完成" }],
      },
    }, "", "assistant-fallback")).toMatchObject({
      event: { assistantMessageId: "assistant-formal" },
      assistantMessage: { id: "assistant-formal" },
    });
  });
});
