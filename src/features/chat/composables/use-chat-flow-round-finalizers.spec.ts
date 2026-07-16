import { describe, expect, it, vi } from "vitest";
import { ref, shallowRef } from "vue";
import type { ChatMessage } from "../../../types/app";
import { useChatFlowRoundFinalizers } from "./use-chat-flow-round-finalizers";

function emptyStreamingAssistant(id: string): ChatMessage {
  return {
    id,
    role: "assistant",
    parts: [{ type: "text", text: "" }],
    providerMeta: {
      _streaming: true,
      _preStreamingStatusText: "正在准备调度...",
    },
  };
}

describe("useChatFlowRoundFinalizers", () => {
  it("终态缺少 assistantMessage 时按正式消息 ID 回读后再收口", async () => {
    const assistant = emptyStreamingAssistant("assistant-1");
    const allMessages = shallowRef<ChatMessage[]>([assistant]);
    const round = ref<{ phase: "queued"; gen: number; messageId: string }>({
      phase: "queued",
      gen: 1,
      messageId: assistant.id,
    });
    const refreshMessageById = vi.fn(async () => {
      allMessages.value = [{
        ...assistant,
        parts: [{ type: "text", text: "完成正文" }],
      }];
      return true;
    });
    const setRound = vi.fn((next: { phase: "idle" } | typeof round.value) => {
      round.value = next as typeof round.value;
    });
    const finalizers = useChatFlowRoundFinalizers({
      allMessages,
      getConversationId: () => "conversation-1",
      refreshMessageById,
      getRound: () => round.value,
      setRound,
      getDeferredRoundCompletion: () => null,
      setDeferredRoundCompletion: vi.fn(),
      latestAssistantText: ref(""),
      toolStatusText: ref(""),
      toolStatusState: ref<"running" | "done" | "failed" | "">(""),
      chatting: ref(true),
      reasoningStartedAtMs: ref(1),
      t: (key: string) => key,
      clearChatErrorText: vi.fn(),
      updateMessageText: vi.fn(),
      finalizeMessage: (messageId: string, finalMessage?: ChatMessage) => {
        allMessages.value = allMessages.value.map((message) => (
          message.id === messageId
            ? {
                ...message,
                ...(finalMessage || {}),
                providerMeta: {},
              }
            : message
        ));
      },
      clearConversationStreamCache: vi.fn(),
      clearFrontendDispatchTimer: vi.fn(),
      setActiveActivationId: vi.fn(),
      setActiveRoundAgentId: vi.fn(),
      onReloadMessages: vi.fn(async () => {}),
      removeMessage: vi.fn(),
      setPendingTerminalEvent: vi.fn(),
      setQueuedStreamingState: vi.fn(),
      sendStartedAtMsByGen: new Map([[1, Date.now()]]),
      getPendingUserDraftId: () => "",
      formatRequestFailed: String,
      setChatErrorText: vi.fn(),
      applyAssistantDeltaToMessage: vi.fn(),
      submitPending: ref(false),
    });

    await finalizers.finalizeQueuedRoundWithoutMessage(1, {
      assistantText: "完成正文",
    });

    expect(refreshMessageById).toHaveBeenCalledWith({
      conversationId: "conversation-1",
      messageId: "assistant-1",
    });
    expect(allMessages.value[0].parts).toEqual([{ type: "text", text: "完成正文" }]);
    expect(allMessages.value[0].providerMeta?._streaming).toBeUndefined();
    expect(round.value.phase).toBe("idle");
  });
});
