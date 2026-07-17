import { onScopeDispose, ref, shallowRef, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AssistantStreamBlock, ChatMentionTarget, ChatMessage, ChatTodoItem } from "../../../types/app";
import { ensureConversationMessageIds } from "../utils/message-id";
import { registerChatFlowRuntime } from "./chat-flow-runtime-registry";
import { useChatFlow } from "./use-chat-flow";
import { DRAFT_USER_ID_PREFIX } from "./use-chat-flow-drafts";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";

type ConversationViewRuntimeOptions = {
  conversationId: Ref<string>;
  apiConfigId: Ref<string>;
  agentId: Ref<string>;
  departmentId: Ref<string>;
  t: (key: string, params?: Record<string, unknown>) => string;
};

export function useConversationViewRuntime(options: ConversationViewRuntimeOptions) {
  const allMessages = shallowRef<ChatMessage[]>([]);
  const chatInput = ref("");
  const selectedMentions = ref<ChatMentionTarget[]>([]);
  const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
  const queuedAttachmentNotices = ref<Array<{ id: string; fileName: string; relativePath: string; mime: string }>>([]);
  const latestUserText = ref("");
  const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
  const latestAssistantText = ref("");
  const toolStatusText = ref("");
  const toolStatusState = ref<"running" | "done" | "failed" | "">("");
  const streamBlocks = ref<AssistantStreamBlock[]>([]);
  const chatErrorText = ref("");
  const chatting = ref(false);
  const trimming = ref(false);
  const submitPending = ref(false);
  const preferredApiConfigId = ref(String(options.apiConfigId.value || "").trim());
  const hasMoreHistory = ref(false);
  const loadingOlderHistory = ref(false);
  const currentTodos = ref<ChatTodoItem[]>([]);
  const planModeEnabled = ref(false);

  function currentConversationId() {
    return String(options.conversationId.value || "").trim();
  }

  async function loadSnapshot() {
    const conversationId = currentConversationId();
    if (!conversationId) {
      allMessages.value = [];
      currentTodos.value = [];
      planModeEnabled.value = false;
      return;
    }
    const snapshot = await invokeTauri<{
      messages?: ChatMessage[];
      preferredApiConfigId?: string | null;
      hasMoreHistory?: boolean;
      shouldBindStream?: boolean;
      streamCache?: ConversationRuntimeStreamCacheSnapshot | null;
      currentTodos?: ChatTodoItem[];
      conversation?: { planModeEnabled?: boolean } | null;
    }>("get_foreground_conversation_light_snapshot", {
      input: { conversationId, agentId: null, limit: 50, resumeProjection: true },
    });
    if (conversationId !== currentConversationId()) return;
    const snapshotApiConfigId = String(snapshot?.preferredApiConfigId || "").trim();
    if (snapshotApiConfigId) preferredApiConfigId.value = snapshotApiConfigId;
    allMessages.value = ensureConversationMessageIds(Array.isArray(snapshot?.messages) ? snapshot.messages : []);
    hasMoreHistory.value = !!snapshot?.hasMoreHistory;
    currentTodos.value = Array.isArray(snapshot?.currentTodos) ? snapshot.currentTodos : [];
    planModeEnabled.value = !!snapshot?.conversation?.planModeEnabled;
    if (snapshot?.shouldBindStream) {
      flow.resumeForegroundRuntimeRound({
        conversationId,
        streamCache: snapshot.streamCache || null,
      });
    }
  }

  async function loadOlderHistory() {
    const conversationId = currentConversationId();
    const oldestMessageId = String(allMessages.value[0]?.id || "").trim();
    if (!conversationId || !oldestMessageId || !hasMoreHistory.value || loadingOlderHistory.value) return;
    loadingOlderHistory.value = true;
    try {
      const result = await invokeTauri<{ messages?: ChatMessage[]; hasMore?: boolean }>("get_active_conversation_messages_before", {
        input: { conversationId, beforeMessageId: oldestMessageId, limit: 50 },
      });
      if (conversationId !== currentConversationId()) return;
      const existingIds = new Set(allMessages.value.map((message) => message.id));
      const incoming = ensureConversationMessageIds(Array.isArray(result?.messages) ? result.messages : [])
        .filter((message) => !existingIds.has(message.id));
      allMessages.value = [...incoming, ...allMessages.value];
      hasMoreHistory.value = !!result?.hasMore;
    } finally {
      loadingOlderHistory.value = false;
    }
  }

  const flow = useChatFlow({
    chatting,
    submitPending,
    trimming,
    isConversationBusy: () => false,
    getSession: () => {
      const apiConfigId = String(preferredApiConfigId.value || options.apiConfigId.value || "").trim();
      const agentId = String(options.agentId.value || "").trim();
      if (!apiConfigId || !agentId) return null;
      return { apiConfigId, agentId, departmentId: String(options.departmentId.value || "").trim() };
    },
    getConversationId: currentConversationId,
    chatInput,
    selectedMentions,
    clipboardImages,
    queuedAttachmentNotices,
    latestUserText,
    latestUserImages,
    latestAssistantText,
    toolStatusText,
    toolStatusState,
    streamBlocks,
    chatErrorText,
    allMessages,
    t: options.t,
    formatRequestFailed: (error) => String(error instanceof Error ? error.message : error || ""),
    removeBinaryPlaceholders: (text) => text,
    invokeSendChatMessage: ({ text, displayText, images, attachments, mentions, session, traceId, onDelta }) =>
      invokeTauri("submit_chat_message", {
        input: {
          payload: {
            text,
            displayText,
            images,
            attachments: attachments && attachments.length > 0 ? attachments : undefined,
            mentions,
          },
          session: {
            apiConfigId: session.apiConfigId,
            agentId: session.agentId,
            departmentId: session.departmentId || null,
            conversationId: session.conversationId || null,
          },
          traceId,
        },
        onDelta,
      }),
    invokeStopChatMessage: ({ session, partialAssistantText, partialStreamBlocks }) =>
      invokeTauri("stop_chat_message", {
        input: {
          session: {
            apiConfigId: session.apiConfigId,
            agentId: session.agentId,
            departmentId: session.departmentId || null,
            conversationId: session.conversationId || null,
          },
          partialAssistantText,
          partialStreamBlocks,
        },
      }),
    refreshMessageById: async ({ conversationId, messageId }) => {
      const message = await invokeTauri<ChatMessage | null>("get_unarchived_conversation_message_by_id", {
        input: { conversationId, messageId },
      });
      if (!message) return false;
      const index = allMessages.value.findIndex((item) => item.id === message.id);
      if (index < 0) return false;
      const next = [...allMessages.value];
      next[index] = message;
      allMessages.value = next;
      return true;
    },
    invokeBindActiveChatViewStream: ({ bindingId, conversationId, onDelta }) =>
      invokeTauri("bind_active_chat_view_stream", {
        input: { bindingId, conversationId: conversationId || null },
        onDelta,
      }),
    invokeUnbindActiveChatViewStream: ({ bindingId }) =>
      invokeTauri("unbind_active_chat_view_stream", { input: { bindingId } }),
    invokeProbeActiveChatViewStream: ({ bindingId, conversationId, probeId }) =>
      invokeTauri<boolean>("probe_active_chat_view_stream", {
        input: { bindingId, conversationId: conversationId || null, probeId },
      }),
    onReloadMessages: loadSnapshot,
    onHistoryFlushed: async ({ conversationId, pendingMessages }) => {
      if (conversationId !== currentConversationId()) return;
      const next = [...allMessages.value];
      const seen = new Set(next.map((message) => message.id));
      for (const message of pendingMessages) {
        if (seen.has(message.id)) continue;
        const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
        const messageMeta = (providerMeta.message_meta || providerMeta.messageMeta || {}) as Record<string, unknown>;
        if (String(messageMeta.kind || "").trim() === "summary_context_seed") {
          next.unshift(message);
          seen.add(message.id);
          continue;
        }
        const draftIndex = message.role === "user"
          ? next.findIndex((item) => item.id.startsWith(DRAFT_USER_ID_PREFIX))
          : -1;
        if (draftIndex >= 0) {
          const draft = next[draftIndex];
          const stableRenderId = String(draft.providerMeta?._stableRenderId || draft.id).trim();
          next[draftIndex] = {
            ...message,
            providerMeta: {
              ...(message.providerMeta || {}),
              _stableRenderId: stableRenderId,
            },
          };
        } else {
          next.push(message);
        }
        seen.add(message.id);
      }
      allMessages.value = next;
    },
    onAssistantMessageCompleted: async ({ conversationId, assistantMessage }) => {
      if (conversationId !== currentConversationId()) return;
      const index = allMessages.value.findIndex((message) => message.id === assistantMessage.id);
      if (index < 0) {
        allMessages.value = [...allMessages.value, assistantMessage];
      } else {
        const next = [...allMessages.value];
        next[index] = assistantMessage;
        allMessages.value = next;
      }
    },
  });

  const runtimeEventHandlers = Object.assign(flow, {
    handleExternalTodosUpdated(payload: unknown) {
      if (!payload || typeof payload !== "object") return;
      const record = payload as { conversationId?: string; currentTodos?: ChatTodoItem[] };
      if (String(record.conversationId || "").trim() !== currentConversationId()) return;
      currentTodos.value = Array.isArray(record.currentTodos) ? record.currentTodos : [];
    },
  });
  const unregister = registerChatFlowRuntime({
    bindingId: flow.bindingId,
    getConversationId: currentConversationId,
    flow: runtimeEventHandlers,
  });

  let loadGeneration = 0;
  watch(options.conversationId, async () => {
    const generation = ++loadGeneration;
    await flow.unbindActiveConversationStream();
    flow.clearForegroundRuntimeState();
    chatErrorText.value = "";
    preferredApiConfigId.value = String(options.apiConfigId.value || "").trim();
    if (!currentConversationId() || generation !== loadGeneration) {
      allMessages.value = [];
      return;
    }
    await loadSnapshot();
    if (generation === loadGeneration) {
      await flow.bindActiveConversationStream(currentConversationId());
    }
  }, { immediate: true });

  onScopeDispose(() => {
    unregister();
    void flow.unbindActiveConversationStream().catch(() => {});
  });

  return {
    flow,
    allMessages,
    chatInput,
    selectedMentions,
    clipboardImages,
    queuedAttachmentNotices,
    latestUserText,
    latestUserImages,
    latestAssistantText,
    toolStatusText,
    toolStatusState,
    streamBlocks,
    chatErrorText,
    chatting,
    submitPending,
    preferredApiConfigId,
    hasMoreHistory,
    loadingOlderHistory,
    currentTodos,
    planModeEnabled,
    send: () => flow.sendChat(),
    stop: () => flow.stopChat(),
    loadSnapshot,
    loadOlderHistory,
  };
}
