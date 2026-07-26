import { computed, ref, type Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import {
  assistantTextFromStreamBlocks,
  normalizeAssistantStreamBlocks,
} from "../../../utils/chat-message-semantics";
import {
  chatMessageRoundMatchesIdentity,
  createChatMessageState,
  reduceChatMessageState,
  type AuthoritativeMessageMergeOptions,
  type ChatAssistantDelta,
  type ChatMessageEvent,
  type ChatMessageState,
} from "../../chat/composables/chat-message-state-machine";
import type {
  SidebarAssistantDeltaPayload,
  SidebarConversationRuntimePayload,
  SidebarStreamCachePayload,
} from "../sidebar-app-types";

type UseSidebarAssistantStreamOptions = {
  messages: Ref<ChatMessage[]>;
  activeAgentId: Ref<string>;
  conversationId: Ref<string>;
};

type PendingAssistantRound = {
  assistantMessageId: string;
  activationId: string;
  requestId: string;
};

type RoundIdentityInput = {
  assistantMessageId?: string;
  activationId?: string;
  requestId?: string;
};

function normalized(value: unknown): string {
  return String(value || "").trim();
}

function normalizeToolStatusState(value: unknown): "running" | "done" | "failed" | "" {
  const status = normalized(value);
  return status === "running" || status === "done" || status === "failed" ? status : "";
}

export function useSidebarAssistantStream(options: UseSidebarAssistantStreamOptions) {
  const toolStatusText = ref("");
  const toolStatusState = ref<"running" | "done" | "failed" | "">("");
  const streamingAssistantMessageId = ref("");
  const streamActivationId = ref("");
  const streamRequestId = ref("");
  const streamRevision = ref("");
  const pendingAssistantRounds = new Map<string, PendingAssistantRound>();
  let messageState = createChatMessageState(options.conversationId.value, options.messages.value);

  const activeMessage = computed(() => options.messages.value.find((message) =>
    normalized(message.id) === streamingAssistantMessageId.value
  ));
  const activeMessageBlocks = computed(() => normalizeAssistantStreamBlocks(activeMessage.value?.contentBlocks));
  const activeMessageText = computed(() => assistantTextFromStreamBlocks(activeMessageBlocks.value));

  function synchronizeMachineInput(): ChatMessageState {
    const conversationId = normalized(options.conversationId.value);
    if (messageState.conversationId !== conversationId) {
      messageState = createChatMessageState(conversationId, options.messages.value);
      pendingAssistantRounds.clear();
      return messageState;
    }
    if (messageState.messages !== options.messages.value) {
      messageState = { ...messageState, messages: options.messages.value };
    }
    return messageState;
  }

  function pendingRoundMatches(
    pending: PendingAssistantRound,
    input: RoundIdentityInput,
  ): boolean {
    const incomingMessageId = normalized(input.assistantMessageId);
    if (incomingMessageId && incomingMessageId !== pending.assistantMessageId) return false;
    const incomingIds = [normalized(input.activationId), normalized(input.requestId)].filter(Boolean);
    if (incomingIds.length === 0) return !!incomingMessageId;
    const pendingIds = [pending.activationId, pending.requestId].filter(Boolean);
    if (pendingIds.length === 0) return !!incomingMessageId && incomingMessageId === pending.assistantMessageId;
    return incomingIds.some((value) => pendingIds.includes(value));
  }

  function findPendingAssistantRound(input: RoundIdentityInput): PendingAssistantRound | undefined {
    synchronizeMachineInput();
    return [...pendingAssistantRounds.values()].find((pending) => pendingRoundMatches(pending, input));
  }

  function trackPendingAssistantRound(
    assistantMessageId: string,
    input?: { activationId?: string; requestId?: string },
  ) {
    synchronizeMachineInput();
    const normalizedMessageId = normalized(assistantMessageId);
    if (!normalizedMessageId) return;
    pendingAssistantRounds.set(normalizedMessageId, {
      assistantMessageId: normalizedMessageId,
      activationId: normalized(input?.activationId),
      requestId: normalized(input?.requestId),
    });
  }

  function pendingAssistantMessageIdForEvent(input: RoundIdentityInput): string {
    return findPendingAssistantRound(input)?.assistantMessageId || "";
  }

  function forgetPendingAssistantRound(input: RoundIdentityInput): string {
    const pending = findPendingAssistantRound(input);
    if (!pending) return "";
    pendingAssistantRounds.delete(pending.assistantMessageId);
    return pending.assistantMessageId;
  }

  function synchronizeRefsFromMachine() {
    const round = messageState.round;
    if (round.phase === "idle") {
      toolStatusText.value = "";
      toolStatusState.value = "";
      streamingAssistantMessageId.value = "";
      streamActivationId.value = "";
      streamRequestId.value = "";
      streamRevision.value = "";
      return;
    }
    streamingAssistantMessageId.value = round.assistantMessageId;
    streamActivationId.value = round.activationId;
    streamRequestId.value = round.requestId;
    streamRevision.value = round.revision;
    const message = messageState.messages.find((item) => normalized(item.id) === round.assistantMessageId);
    const meta = (message?.providerMeta || {}) as Record<string, unknown>;
    toolStatusText.value = String(meta._toolStatusText || "");
    toolStatusState.value = normalizeToolStatusState(meta._toolStatusState);
  }

  function dispatchMessageEvent(event: ChatMessageEvent): ChatMessageState {
    synchronizeMachineInput();
    messageState = reduceChatMessageState(messageState, event);
    options.messages.value = messageState.messages;
    if (event.type === "round_started" && chatMessageRoundMatchesIdentity(messageState.round, event)) {
      pendingAssistantRounds.delete(normalized(event.assistantMessageId));
    }
    synchronizeRefsFromMachine();
    return messageState;
  }

  function messageEventTargetsActiveRound(
    event: Extract<ChatMessageEvent, { type: "round_started" | "round_finished" | "round_failed" }>,
  ): boolean {
    synchronizeMachineInput();
    return chatMessageRoundMatchesIdentity(messageState.round, event);
  }

  function messageRoundIsSettling(
    messageId: string,
    identity?: { activationId?: string; requestId?: string },
  ): boolean {
    synchronizeMachineInput();
    const normalizedMessageId = normalized(messageId);
    return messageState.round.phase === "settling"
      && messageState.round.assistantMessageId === normalizedMessageId
      && chatMessageRoundMatchesIdentity(messageState.round, {
        assistantMessageId: normalizedMessageId,
        activationId: identity?.activationId,
        requestId: identity?.requestId,
      });
  }

  function clearStreamingState() {
    synchronizeMachineInput();
    messageState = createChatMessageState(options.conversationId.value, options.messages.value);
    synchronizeRefsFromMachine();
  }

  function replaceHistory(messages: ChatMessage[]) {
    const conversationId = normalized(options.conversationId.value);
    if (!conversationId) {
      options.messages.value = Array.isArray(messages) ? messages : [];
      clearStreamingState();
      return;
    }
    dispatchMessageEvent({
      type: "history_replaced",
      conversationId,
      messages: Array.isArray(messages) ? messages : [],
    });
  }

  function mergeAuthoritativeMessages(
    messages: ChatMessage[],
    mergeOptions?: AuthoritativeMessageMergeOptions,
  ) {
    const conversationId = normalized(options.conversationId.value);
    if (!conversationId || !Array.isArray(messages) || messages.length === 0) return;
    dispatchMessageEvent({
      type: "authoritative_messages_merged",
      conversationId,
      messages,
      options: mergeOptions,
    });
  }

  function startStreamingMessage(
    messageId: string,
    input?: {
      activationId?: string;
      requestId?: string;
      revision?: string;
      startedAt?: string;
      startedAtMs?: number;
      speakerAgentId?: string;
      statusText?: string;
      phase?: "waiting" | "streaming";
    },
  ) {
    const assistantMessageId = normalized(messageId);
    const conversationId = normalized(options.conversationId.value);
    if (!assistantMessageId || !conversationId) return;
    dispatchMessageEvent({
      type: "round_started",
      conversationId,
      assistantMessageId,
      activationId: normalized(input?.activationId) || undefined,
      requestId: normalized(input?.requestId) || undefined,
      revision: normalized(input?.revision) || undefined,
      startedAt: normalized(input?.startedAt) || new Date().toISOString(),
      startedAtMs: input?.startedAtMs,
      speakerAgentId: normalized(input?.speakerAgentId || options.activeAgentId.value) || undefined,
      statusText: input?.statusText,
      phase: input?.phase || "waiting",
    });
  }

  function writeStreamCacheToMessage(cache: SidebarStreamCachePayload) {
    const messageId = normalized(cache.persistedAssistantMessageId || streamingAssistantMessageId.value);
    const conversationId = normalized(options.conversationId.value);
    if (!messageId || !conversationId) return;
    dispatchMessageEvent({
      type: "assistant_stream_snapshot",
      conversationId,
      assistantMessageId: messageId,
      snapshot: {
        activationId: cache.activationId,
        requestId: cache.requestId,
        updatedAt: cache.updatedAt,
        startedAt: new Date().toISOString(),
        assistantText: cache.assistantText,
        toolStatusText: cache.toolStatusText,
        toolStatusState: cache.toolStatusState,
        streamBlocks: cache.streamBlocks,
        persistedAssistantMessageId: messageId,
        speakerAgentId: options.activeAgentId.value,
      },
    });
  }

  function applyAssistantDeltaEvent(event: ChatAssistantDelta) {
    const conversationId = normalized(options.conversationId.value);
    if (!conversationId) return;
    dispatchMessageEvent({
      type: "assistant_delta",
      conversationId,
      event: {
        ...event,
        assistantMessageId: normalized(
          event.assistantMessageId || streamingAssistantMessageId.value,
        ) || undefined,
      },
    });
  }

  function finishStreamingMessage(messageId: string, finalMessage?: ChatMessage) {
    const conversationId = normalized(options.conversationId.value);
    if (!conversationId) return;
    dispatchMessageEvent({
      type: "round_finished",
      conversationId,
      assistantMessageId: normalized(messageId) || undefined,
      assistantMessage: finalMessage,
    });
  }

  function applyRuntimeStreamCache(runtime: SidebarConversationRuntimePayload | null | undefined) {
    if (!runtime?.streamCache) return;
    writeStreamCacheToMessage(runtime.streamCache);
  }

  function applyAssistantToolStatusEvent(event: NonNullable<SidebarAssistantDeltaPayload["event"]>) {
    applyAssistantDeltaEvent({
      kind: "tool_status",
      message: event.message,
      toolStatus: event.toolStatus,
    });
  }

  function applyAssistantToolEvent(message: string) {
    applyAssistantDeltaEvent({ kind: "assistant_tool_event", message });
  }

  function appendAssistantTextDelta(delta: string) {
    applyAssistantDeltaEvent({ delta });
  }

  return {
    activeMessageBlocks,
    activeMessageText,
    streamingAssistantMessageId,
    streamActivationId,
    streamRequestId,
    streamRevision,
    toolStatusState,
    toolStatusText,
    appendAssistantTextDelta,
    applyAssistantDeltaEvent,
    applyAssistantToolEvent,
    applyAssistantToolStatusEvent,
    applyRuntimeStreamCache,
    clearStreamingState,
    dispatchMessageEvent,
    finishStreamingMessage,
    messageEventTargetsActiveRound,
    messageRoundIsSettling,
    pendingAssistantMessageIdForEvent,
    trackPendingAssistantRound,
    forgetPendingAssistantRound,
    mergeAuthoritativeMessages,
    replaceHistory,
    startStreamingMessage,
    writeStreamCacheToMessage,
  };
}
