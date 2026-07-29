import { describe, expect, it, vi } from "vitest";
import { ref, shallowRef } from "vue";
import type { ChatMessage } from "../src/types/app";
import { useChatConversationSync } from "../src/features/chat/composables/use-chat-conversation-sync";
import { useChatConversationMessageUtils } from "../src/features/chat/composables/use-chat-conversation-message-utils";

const hoisted = vi.hoisted(() => ({
  invokeTauriMock: vi.fn(),
}));

vi.mock("../src/services/tauri-api", () => ({
  invokeTauri: hoisted.invokeTauriMock,
}));

function textMessage(id: string, role: ChatMessage["role"], text: string): ChatMessage {
  return {
    id,
    role,
    parts: [{ type: "text", text }],
  };
}

function buildConversationSync(overrides: {
  currentConversationId?: string;
  previousMessages?: ChatMessage[];
  readConversationStreamCache?: (conversationId?: string | null) => any;
} = {}) {
  const allMessages = shallowRef<ChatMessage[]>(overrides.previousMessages ?? []);

  const sync = useChatConversationSync({
    BACKGROUND_CONVERSATION_CACHE_LIMIT: 20,
    OLDER_HISTORY_PAGE_SIZE: 20,
    currentChatConversationId: ref(overrides.currentConversationId ?? ""),
    currentChatPreferredApiConfigId: ref(""),
    currentChatTodos: ref([]),
    currentForegroundAgentId: ref(""),
    currentForegroundApiConfigId: ref(""),
    detachedChatConversationId: ref(""),
    detachedChatWindow: ref(false),
    tauriWindowLabel: ref("chat"),
    unarchivedConversations: ref([]),
    allMessages,
    hasMoreBackendHistory: ref(false),
    loadingOlderConversationHistory: ref(false),
    foregroundTailLatestReady: ref(false),
    conversationMessageCache: ref({}),
    backgroundConversationBadgeMap: ref({}),
    ensureConversationMessageIds: (messages: any[]) => messages,
    clearPendingManualScrollToBottom: vi.fn(),
    triggerConversationScrollToBottom: vi.fn(),
    getPendingManualScrollToBottomConversationId: () => "",
    getPendingManualScrollToBottomRequestId: () => "",
    loadAllMessages: vi.fn(),
    getChatFlow: () => ({
      resumeForegroundStreamingRound: vi.fn(),
      bindActiveConversationStream: vi.fn(async () => {}),
      resumeForegroundRuntimeRound: vi.fn(),
    }),
    readConversationStreamCache: overrides.readConversationStreamCache || (() => null),
    setStatusError: vi.fn(),
    perfNow: () => 0,
    tr: (key: string) => key,
  });

  return {
    allMessages,
    sync,
  };
}

describe("useChatConversationMessageUtils", () => {
  const utils = useChatConversationMessageUtils({
    ensureConversationMessageIds: (messages) => messages,
  });

  const localMessage: ChatMessage = {
    ...textMessage("assistant-1", "assistant", "打断时保留的正文"),
    providerMeta: {
      contextUsagePercent: 10,
      contextUsageRatio: 0.1,
      model: "local-model",
    },
  };
  const incomingMessage: ChatMessage = {
    ...textMessage("assistant-1", "assistant", "后端稍后落盘的更长正文"),
    providerMeta: {
      contextUsagePercent: 42,
      contextUsageRatio: 0.42,
      effectivePromptTokens: 420,
      providerPromptTokens: 400,
      contextWindowTokens: 1000,
      model: "remote-model",
    },
  };

  it.each([
    ["mergeMessagesIntoTimeline", () => utils.mergeMessagesIntoTimeline([localMessage], [incomingMessage])],
    ["replaceConversationMessage", () => utils.replaceConversationMessage([localMessage], incomingMessage)],
  ])("keeps visible content but refreshes authoritative usage through %s", (_name, applyUpdate) => {
    const [result] = applyUpdate();

    expect(result.parts).toEqual(localMessage.parts);
    expect(result.providerMeta).toMatchObject({
      contextUsagePercent: 42,
      contextUsageRatio: 0.42,
      effectivePromptTokens: 420,
      providerPromptTokens: 400,
      contextWindowTokens: 1000,
      model: "local-model",
    });
  });
});
