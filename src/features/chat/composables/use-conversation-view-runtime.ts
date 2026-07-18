import { computed, onScopeDispose, ref, shallowRef, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AssistantStreamBlock, ChatMentionTarget, ChatMessage, ChatTodoItem } from "../../../types/app";
import { ensureConversationMessageIds } from "../utils/message-id";
import { preserveStableRenderId } from "../utils/stable-render-id";
import { registerChatFlowRuntime } from "./chat-flow-runtime-registry";
import type { ExclusiveChatViewSubscriptionSlot } from "./exclusive-chat-view-subscription-slot";
import { reconcileAuthoritativeConversationMessage } from "./chat-message-reconciliation";
import {
  createLatestTaskRunner,
  reconcileForegroundConversation as reconcileChatForegroundConversation,
  runForegroundSnapshotBindingTransaction,
} from "./chat-foreground-coordinator";
import { useChatFlow } from "./use-chat-flow";
import { DRAFT_ASSISTANT_ID_PREFIX, DRAFT_USER_ID_PREFIX } from "./use-chat-flow-drafts";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";

type ConversationViewRuntimeOptions = {
  conversationId: Ref<string>;
  apiConfigId: Ref<string>;
  agentId: Ref<string>;
  departmentId: Ref<string>;
  subscriptionSlot?: ExclusiveChatViewSubscriptionSlot;
  t: (key: string, params?: Record<string, unknown>) => string;
};

type ConversationRuntimeState = "idle" | "assistant_streaming" | "organizing_context";

type ConversationLightSnapshot = {
  conversationId?: string;
  messages?: ChatMessage[];
  preferredApiConfigId?: string | null;
  hasMoreHistory?: boolean;
  runtimeState?: ConversationRuntimeState | null;
  shouldBindStream?: boolean;
  streamCache?: ConversationRuntimeStreamCacheSnapshot | null;
  resumeProjectionAuthoritative?: boolean;
  currentTodos?: ChatTodoItem[];
  conversation?: { planModeEnabled?: boolean } | null;
};

type ConversationRuntimeSnapshot = {
  conversationId?: string;
  runtimeState?: ConversationRuntimeState;
  isProcessing?: boolean;
  hasPendingQueue?: boolean;
  pendingQueueCount?: number;
  streamCache?: ConversationRuntimeStreamCacheSnapshot | null;
};

export function useConversationViewRuntime(options: ConversationViewRuntimeOptions) {
  const allMessages = shallowRef<ChatMessage[]>([]);
  const chatInput = ref("");
  const selectedMentions = ref<ChatMentionTarget[]>([]);
  const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
  const queuedAttachmentNotices = ref<Array<{ id: string; fileName: string; path: string; mime: string }>>([]);
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
  const runtimeState = ref<ConversationRuntimeState>("idle");
  const foregroundSyncing = ref(false);
  const conversationBusy = computed(() =>
    submitPending.value
    || chatting.value
    || runtimeState.value === "assistant_streaming"
    || runtimeState.value === "organizing_context"
  );
  let snapshotRequestSequence = 0;
  let disposed = false;

  function currentConversationId() {
    if (disposed) return "";
    return String(options.conversationId.value || "").trim();
  }

  function messageCreatedAtMs(message?: ChatMessage): number | null {
    const raw = String(message?.createdAt || "").trim();
    if (!raw) return null;
    const value = Date.parse(raw);
    return Number.isFinite(value) ? value : null;
  }

  function insertMessageIntoTimeline(messages: ChatMessage[], incoming: ChatMessage): ChatMessage[] {
    const incomingAt = messageCreatedAtMs(incoming);
    if (incomingAt === null) return [...messages, incoming];
    const index = messages.findIndex((message) => {
      const existingAt = messageCreatedAtMs(message);
      return existingAt !== null && existingAt > incomingAt;
    });
    if (index < 0) return [...messages, incoming];
    return [...messages.slice(0, index), incoming, ...messages.slice(index)];
  }

  function mergeAuthoritativeMessages(messages: ChatMessage[], incomingMessages: ChatMessage[]): ChatMessage[] {
    let next = [...messages];
    for (const incoming of ensureConversationMessageIds(incomingMessages)) {
      const messageId = String(incoming.id || "").trim();
      const existingIndex = messageId
        ? next.findIndex((message) => String(message.id || "").trim() === messageId)
        : -1;
      if (existingIndex >= 0) {
        const replacement = reconcileAuthoritativeConversationMessage(next[existingIndex], incoming);
        next = next.map((message, index) => index === existingIndex ? replacement : message);
      } else {
        next = insertMessageIntoTimeline(next, incoming);
      }
    }
    return next;
  }

  function currentFormalTailMessageId(): string {
    const formalMessages = allMessages.value.filter((message) => {
      const messageId = String(message.id || "").trim();
      return !!messageId
        && !messageId.startsWith(DRAFT_USER_ID_PREFIX)
        && !messageId.startsWith(DRAFT_ASSISTANT_ID_PREFIX);
    });
    return String(formalMessages[formalMessages.length - 1]?.id || "").trim();
  }

  function frontendConversationIsStreaming(): boolean {
    const phase = String(flow.frontendRoundPhase.value || "").trim();
    return chatting.value || phase === "queued" || phase === "waiting" || phase === "streaming";
  }

  function applySnapshot(snapshot: ConversationLightSnapshot, preserveExistingHistory: boolean) {
    const incomingMessages = ensureConversationMessageIds(Array.isArray(snapshot?.messages) ? snapshot.messages : []);
    allMessages.value = preserveExistingHistory
      ? mergeAuthoritativeMessages(allMessages.value, incomingMessages)
      : incomingMessages.map((message) => {
        const previous = allMessages.value.find((item) => item.id === message.id);
        return preserveStableRenderId(message, previous);
      });
    const snapshotApiConfigId = String(snapshot?.preferredApiConfigId || "").trim();
    if (snapshotApiConfigId) preferredApiConfigId.value = snapshotApiConfigId;
    hasMoreHistory.value = !!snapshot?.hasMoreHistory;
    currentTodos.value = Array.isArray(snapshot?.currentTodos) ? snapshot.currentTodos : [];
    planModeEnabled.value = !!snapshot?.conversation?.planModeEnabled;
    runtimeState.value = snapshot?.runtimeState || (snapshot?.shouldBindStream ? "assistant_streaming" : "idle");
  }

  async function requestSnapshot(conversationId: string) {
    const requestSequence = ++snapshotRequestSequence;
    if (!conversationId) {
      allMessages.value = [];
      currentTodos.value = [];
      planModeEnabled.value = false;
      runtimeState.value = "idle";
      return null;
    }
    const snapshot = await invokeTauri<ConversationLightSnapshot>("get_foreground_conversation_light_snapshot", {
      input: { conversationId, agentId: null, limit: 50, resumeProjection: true },
    });
    const snapshotConversationId = String(snapshot?.conversationId || conversationId).trim();
    if (
      requestSequence !== snapshotRequestSequence
      || conversationId !== currentConversationId()
      || snapshotConversationId !== conversationId
    ) return null;
    return snapshot;
  }

  let foregroundSyncSequence = 0;
  // 同一 ConversationView 的解绑命令只按 bindingId 生效；恢复事务必须串行，
  // 否则较早的解绑可能晚于新绑定完成，并把刚建立的 Channel 一并移除。
  let foregroundSyncQueue: Promise<void> = Promise.resolve();

  function synchronizeConversation(
    conversationId: string,
    syncOptions: { clearRuntime: boolean; preserveExistingHistory: boolean },
  ): Promise<void> {
    const syncSequence = ++foregroundSyncSequence;
    const task = foregroundSyncQueue.then(async () => {
      if (disposed || !conversationId || conversationId !== currentConversationId()) return;
      foregroundSyncing.value = true;
      try {
        await runForegroundSnapshotBindingTransaction({
          conversationId,
          isCurrent: () => !disposed
            && conversationId === currentConversationId()
            && syncSequence === foregroundSyncSequence,
          clearRuntime: () => {
            if (syncOptions.clearRuntime) flow.clearForegroundRuntimeState();
          },
          unbind: flow.unbindActiveConversationStream,
          requestSnapshot: () => requestSnapshot(conversationId),
          applySnapshot: (snapshot) => applySnapshot(snapshot, syncOptions.preserveExistingHistory),
          bind: () => flow.bindActiveConversationStream(conversationId, true),
          resume: (snapshot) => {
            flow.resumeForegroundRuntimeRound({
              conversationId,
              streamCache: snapshot.streamCache || null,
              statusText: options.t("chat.statusWaitingReply"),
              reason: "conversation_view_snapshot_ready",
            });
          },
          onUnbindError: (error) => {
            console.warn("[追问会话] 取消流式通道绑定失败", { conversationId, error });
          },
        });
      } finally {
        if (syncSequence === foregroundSyncSequence) {
          foregroundSyncing.value = false;
        }
      }
    });
    foregroundSyncQueue = task.catch((error) => {
      console.error("[追问会话] 前台同步失败", { conversationId, error });
      if (syncSequence === foregroundSyncSequence) {
        foregroundSyncing.value = false;
        chatErrorText.value = String(error instanceof Error ? error.message : error || "");
      }
    });
    return foregroundSyncQueue;
  }

  async function loadSnapshot() {
    const conversationId = currentConversationId();
    if (!conversationId) {
      ++snapshotRequestSequence;
      allMessages.value = [];
      currentTodos.value = [];
      planModeEnabled.value = false;
      runtimeState.value = "idle";
      return;
    }
    await synchronizeConversation(conversationId, {
      clearRuntime: true,
      preserveExistingHistory: true,
    });
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

  async function refreshMessageById(conversationId: string, messageId: string) {
    const message = await invokeTauri<ChatMessage | null>("get_unarchived_conversation_message_by_id", {
      input: { conversationId, messageId },
    });
    if (!message || conversationId !== currentConversationId()) return false;
    const index = allMessages.value.findIndex((item) => item.id === message.id);
    if (index < 0) return false;
    const next = [...allMessages.value];
    next[index] = reconcileAuthoritativeConversationMessage(next[index], message, { forceReplace: true });
    allMessages.value = next;
    return true;
  }

  const flow = useChatFlow({
    chatting,
    submitPending,
    trimming,
    isConversationBusy: () => foregroundSyncing.value || conversationBusy.value,
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
    invokeSendChatMessage: ({ text, displayText, parts, mentions, session, traceId, onDelta }) =>
      invokeTauri("submit_chat_message", {
        input: {
          payload: {
            text,
            displayText,
            parts,
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
    refreshMessageById: ({ conversationId, messageId }) => refreshMessageById(conversationId, messageId),
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
    coordinateActiveConversationStreamBind: ({ bindingId, conversationId, bind, unbind }) => {
      if (!options.subscriptionSlot) return bind();
      return options.subscriptionSlot.acquire({
        ownerId: bindingId,
        conversationId,
        bind,
        unbind,
      });
    },
    onReloadMessages: loadSnapshot,
    onHistoryFlushed: async ({ conversationId, pendingMessages }) => {
      if (conversationId !== currentConversationId()) return;
      const next = [...allMessages.value];
      for (const message of pendingMessages) {
        const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
        const messageMeta = (providerMeta.message_meta || providerMeta.messageMeta || {}) as Record<string, unknown>;
        const existingIndex = next.findIndex((item) => item.id === message.id);
        if (existingIndex >= 0) {
          next[existingIndex] = reconcileAuthoritativeConversationMessage(next[existingIndex], message);
          continue;
        }
        if (String(messageMeta.kind || "").trim() === "summary_context_seed") {
          next.unshift(message);
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
          const merged = insertMessageIntoTimeline(next, message);
          next.splice(0, next.length, ...merged);
        }
      }
      allMessages.value = next;
    },
    onAssistantMessageCompleted: async ({ conversationId, assistantMessage }) => {
      if (conversationId !== currentConversationId()) return;
      const index = allMessages.value.findIndex((message) => message.id === assistantMessage.id);
      if (index < 0) {
        allMessages.value = insertMessageIntoTimeline(allMessages.value, assistantMessage);
      } else {
        const next = [...allMessages.value];
        next[index] = reconcileAuthoritativeConversationMessage(next[index], assistantMessage);
        allMessages.value = next;
      }
    },
  });

  async function requestRuntimeSnapshot(conversationId: string) {
    return invokeTauri<ConversationRuntimeSnapshot>("get_conversation_runtime_snapshot", {
      conversationId,
    });
  }

  async function requestLatestFormalTailMessageId(conversationId: string) {
    const snapshot = await invokeTauri<{ lastMessageId?: string | null }>("get_foreground_conversation_freshness_snapshot", {
      input: { conversationId, agentId: null },
    });
    return String(snapshot?.lastMessageId || "").trim();
  }

  async function reconcileForegroundConversation(reason: string) {
    const conversationId = currentConversationId();
    if (!conversationId || foregroundSyncing.value) return;
    await reconcileChatForegroundConversation({
      conversationId,
      isCurrent: () => !disposed && conversationId === currentConversationId(),
      requestRuntimeSnapshot: () => requestRuntimeSnapshot(conversationId),
      applyRuntimeState: (snapshot) => {
        runtimeState.value = (snapshot.runtimeState as ConversationRuntimeState) || "idle";
      },
      frontendStreaming: frontendConversationIsStreaming,
      readFrontendStreamCache: () => flow.readConversationStreamCache?.(conversationId),
      probeStream: () => flow.probeBoundChannel(conversationId),
      readCurrentFormalTailMessageId: currentFormalTailMessageId,
      requestLatestFormalTailMessageId: () => requestLatestFormalTailMessageId(conversationId),
      refreshTargetMessage: (messageId) => refreshMessageById(conversationId, messageId),
      resumeStream: async (snapshot) => {
        await flow.bindActiveConversationStream(conversationId, true);
        return flow.resumeForegroundRuntimeRound({
          conversationId,
          streamCache: snapshot.streamCache || null,
          reason: `foreground_${reason}`,
        }) > 0;
      },
      finalizeTargetRefresh: async () => {
        flow.clearForegroundRuntimeState();
        await flow.unbindActiveConversationStream().catch(() => {});
        runtimeState.value = "idle";
      },
      reloadConversation: () => synchronizeConversation(conversationId, {
        clearRuntime: true,
        preserveExistingHistory: true,
      }),
    });
  }

  const foregroundRecoveryRunner = createLatestTaskRunner(async (reason: string) => {
    await reconcileForegroundConversation(reason);
  });

  function scheduleForegroundRecovery(reason = "unknown") {
    return foregroundRecoveryRunner.run(reason).catch((error) => {
        console.error("[追问会话] 前台恢复失败", {
          conversationId: currentConversationId(),
          error,
        });
      });
  }

  const handleExternalRoundStarted = flow.handleExternalRoundStarted.bind(flow);
  const handleExternalRoundCompleted = flow.handleExternalRoundCompleted.bind(flow);
  const handleExternalRoundFailed = flow.handleExternalRoundFailed.bind(flow);
  const runtimeEventHandlers = Object.assign({}, flow, {
    async handleExternalRoundStarted(payload: unknown) {
      runtimeState.value = "assistant_streaming";
      await handleExternalRoundStarted(payload);
    },
    async handleExternalRoundCompleted(payload: unknown) {
      await handleExternalRoundCompleted(payload);
      if (!frontendConversationIsStreaming()) {
        runtimeState.value = "idle";
      }
    },
    async handleExternalRoundFailed(payload: unknown) {
      await handleExternalRoundFailed(payload);
      if (!frontendConversationIsStreaming()) {
        runtimeState.value = "idle";
      }
    },
    handleExternalMessageAppended(payload: unknown) {
      if (!payload || typeof payload !== "object") return;
      const record = payload as { conversationId?: string; message?: ChatMessage };
      if (String(record.conversationId || "").trim() !== currentConversationId() || !record.message) return;
      allMessages.value = mergeAuthoritativeMessages(allMessages.value, [record.message]);
    },
    handleExternalMessagesAfterSynced(payload: unknown) {
      if (!payload || typeof payload !== "object") return;
      const record = payload as { conversationId?: string; messages?: ChatMessage[]; error?: unknown };
      if (String(record.conversationId || "").trim() !== currentConversationId() || record.error) return;
      allMessages.value = mergeAuthoritativeMessages(
        allMessages.value,
        Array.isArray(record.messages) ? record.messages : [],
      );
    },
    handleExternalRuntimeStateUpdated(payload: unknown) {
      if (!payload || typeof payload !== "object") return;
      const record = payload as { conversationId?: string; runtimeState?: ConversationRuntimeState };
      if (String(record.conversationId || "").trim() !== currentConversationId()) return;
      const nextRuntimeState = String(record.runtimeState || "").trim();
      if (
        nextRuntimeState !== "idle"
        && nextRuntimeState !== "assistant_streaming"
        && nextRuntimeState !== "organizing_context"
      ) return;
      runtimeState.value = nextRuntimeState;
      const frontendStreaming = frontendConversationIsStreaming();
      const shouldRecover = nextRuntimeState === "idle"
        ? frontendStreaming
        : !frontendStreaming || !flow.hasActiveBoundDeltaChannel(currentConversationId());
      if (shouldRecover) void scheduleForegroundRecovery();
    },
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

  watch(options.conversationId, async () => {
    const conversationId = currentConversationId();
    chatErrorText.value = "";
    preferredApiConfigId.value = String(options.apiConfigId.value || "").trim();
    runtimeState.value = "idle";
    if (!conversationId) {
      ++foregroundSyncSequence;
      ++snapshotRequestSequence;
      flow.clearForegroundRuntimeState();
      await flow.unbindActiveConversationStream().catch(() => {});
      return;
    }
    await synchronizeConversation(conversationId, {
      clearRuntime: true,
      preserveExistingHistory: false,
    });
  }, { immediate: true });

  function handleForegroundWake(event: Event) {
    if (document.visibilityState === "hidden") return;
    void scheduleForegroundRecovery(event.type);
  }

  window.addEventListener("focus", handleForegroundWake);
  document.addEventListener("visibilitychange", handleForegroundWake);

  onScopeDispose(() => {
    disposed = true;
    foregroundRecoveryRunner.cancel();
    ++foregroundSyncSequence;
    ++snapshotRequestSequence;
    window.removeEventListener("focus", handleForegroundWake);
    document.removeEventListener("visibilitychange", handleForegroundWake);
    unregister();
    const unbindPromise = flow.unbindActiveConversationStream().catch(() => {});
    if (options.subscriptionSlot) {
      void options.subscriptionSlot.release(flow.bindingId, unbindPromise).catch(() => {});
    } else {
      void unbindPromise;
    }
  });

  return {
    flow: runtimeEventHandlers,
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
    runtimeState,
    conversationBusy,
    foregroundSyncing,
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
