import { describe, expect, it } from "vitest";
import { sidebarAssistantDeltaToMessageEvent } from "../../sidebar/composables/sidebar-chat-message-adapter";
import { tauriAssistantDeltaToMessageEvent } from "./chat-message-tauri-adapter";

describe("Tauri and Sidebar message adapters", () => {
  it("produce equivalent canonical delta events for the same stream payload", () => {
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
    const tauriEvent = tauriAssistantDeltaToMessageEvent({
      kind: "tool_status",
      message: "正在执行",
      toolStatus: "running",
      streamCache,
    }, "conversation-1", "assistant-1");
    const sidebarEvent = sidebarAssistantDeltaToMessageEvent({
      conversationId: "conversation-1",
      event: {
        kind: "tool_status",
        message: "正在执行",
        toolStatus: "running",
        streamCache,
      },
    }, "", "assistant-1");

    expect(tauriEvent?.type).toBe("assistant_delta");
    expect(sidebarEvent?.type).toBe("assistant_delta");
    expect(tauriEvent?.conversationId).toBe(sidebarEvent?.conversationId);
    const tauriCanonical = JSON.parse(JSON.stringify(tauriEvent?.event || {}));
    const sidebarCanonical = JSON.parse(JSON.stringify(sidebarEvent?.event || {}));
    expect(tauriCanonical).toEqual(sidebarCanonical);
  });
});
