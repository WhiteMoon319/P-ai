import { describe, expect, it, vi } from "vitest";
import {
  chatFlowRuntimesForConversation,
  registerChatFlowRuntime,
  type ChatFlowRuntimeEventHandlers,
} from "./chat-flow-runtime-registry";

function createFlow(): ChatFlowRuntimeEventHandlers {
  return {
    handleExternalAssistantDelta: vi.fn(),
    handleExternalHistoryFlushed: vi.fn(),
    handleExternalRoundCompleted: vi.fn(),
    handleExternalRoundFailed: vi.fn(),
    handleExternalRoundStarted: vi.fn(),
    handleExternalStreamRebindRequired: vi.fn(),
  };
}

describe("chatFlowRuntimeRegistry", () => {
  it("按 conversationId 找到同一窗口中的不同 runtime", () => {
    let firstConversationId = "conversation-a";
    const firstFlow = createFlow();
    const secondFlow = createFlow();
    const unregisterFirst = registerChatFlowRuntime({
      bindingId: "view-main",
      getConversationId: () => firstConversationId,
      flow: firstFlow,
    });
    const unregisterSecond = registerChatFlowRuntime({
      bindingId: "view-side",
      getConversationId: () => "conversation-b",
      flow: secondFlow,
    });

    expect(chatFlowRuntimesForConversation("conversation-a")).toEqual([firstFlow]);
    expect(chatFlowRuntimesForConversation("conversation-b")).toEqual([secondFlow]);

    firstConversationId = "conversation-c";
    expect(chatFlowRuntimesForConversation("conversation-a")).toEqual([]);
    expect(chatFlowRuntimesForConversation("conversation-c")).toEqual([firstFlow]);

    unregisterFirst();
    unregisterSecond();
  });

  it("同一 conversationId 可以同时路由给多个 view runtime", () => {
    const firstFlow = createFlow();
    const secondFlow = createFlow();
    const unregisterFirst = registerChatFlowRuntime({
      bindingId: "view-one",
      getConversationId: () => "conversation-a",
      flow: firstFlow,
    });
    const unregisterSecond = registerChatFlowRuntime({
      bindingId: "view-two",
      getConversationId: () => "conversation-a",
      flow: secondFlow,
    });

    expect(chatFlowRuntimesForConversation("conversation-a")).toEqual([firstFlow, secondFlow]);

    unregisterFirst();
    unregisterSecond();
  });
});
