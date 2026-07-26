import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMentionTarget, ChatMessage } from "../../../types/app";
import {
  assistantTextFromStreamBlocks,
  assistantContentBlocksFromMessage,
  normalizeAssistantStreamBlocks,
  normalizeChatActivityItems,
  streamBlocksToToolCalls,
} from "../../../utils/chat-message-semantics";
import { readMessagePlainText, messageHasVisibleContent } from "./use-chat-flow-utils";
import { messageWithStableRenderId } from "../utils/stable-render-id";
import {
  assistantMessageHasCanonicalVisibleContent,
  createChatMessageState,
  reconcileCompletedAssistantMessage,
  reduceChatMessageState,
  type ChatMessageEvent,
  type ChatMessageState,
} from "./chat-message-state-machine";
import {
  tauriAssistantDeltaToMessageEvent,
  tauriRoundStartedToMessageEvent,
} from "./chat-message-tauri-adapter";
import type { AssistantDeltaEvent, RoundStartedPayload } from "./use-chat-flow-events";

export const DRAFT_ASSISTANT_ID_PREFIX = "__draft_assistant__:";
export const DRAFT_USER_ID_PREFIX = "__draft_user__:";

type UpdateMessageTextOptions = {
  preserveActivityProjection?: boolean;
};

function messageHasActivityEvents(message: ChatMessage): boolean {
  if (normalizeChatActivityItems(message.activityItems).length > 0) return true;
  if (!Array.isArray(message.toolCall)) return false;
  return message.toolCall.some((event) => {
    const raw = event && typeof event === "object" ? event as Record<string, unknown> : null;
    if (!raw) return false;
    if (String(raw.reasoning_content || "").trim()) return true;
    return Array.isArray(raw.tool_calls) && raw.tool_calls.length > 0;
  });
}

function assistantMessageHasVisibleProgress(message?: ChatMessage | null): boolean {
  if (!message) return false;
  if (readMessagePlainText(message).trim()) return true;
  if (messageHasActivityEvents(message)) return true;
  const meta = (message.providerMeta || {}) as Record<string, unknown>;
  if (String(meta._streamTail || "").trim()) return true;
  if (Array.isArray(meta._streamSegments) && meta._streamSegments.some((item) => String(item || "").trim())) {
    return true;
  }
  const streamBlocks = assistantContentBlocksFromMessage(message);
  return streamBlocks.length > 0 || !!assistantTextFromStreamBlocks(streamBlocks).trim();
}

type UseChatFlowDraftsOptions = {
  allMessages: Ref<ChatMessage[]>;
  latestUserText: Ref<string>;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  getActiveRoundAgentId?: () => string;
  getConversationId?: () => string;
  getSendStartedAtMs: (gen: number) => number;
  getActiveHistoryMessageCount: () => number;
  getFrontendDispatchStartedAtMs: () => number;
  currentFrontendDispatchElapsedMs: () => number;
};

export function useChatFlowDrafts(options: UseChatFlowDraftsOptions) {
  let pendingUserDraftId = "";
  const pendingUserDraftIdByGen = new Map<number, string>();
  let messageState = createChatMessageState(
    String(options.getConversationId ? options.getConversationId() : "").trim() || "__foreground__",
    options.allMessages.value,
  );

  function machineConversationId(): string {
    return String(options.getConversationId ? options.getConversationId() : "").trim() || "__foreground__";
  }

  function synchronizeMachineInput(): ChatMessageState {
    const conversationId = machineConversationId();
    if (messageState.conversationId !== conversationId) {
      messageState = createChatMessageState(conversationId, options.allMessages.value);
      return messageState;
    }
    if (messageState.messages !== options.allMessages.value) {
      messageState = { ...messageState, messages: options.allMessages.value };
    }
    if (messageState.round.phase !== "idle") {
      const activeMessage = messageState.messages.find((message) => message.id === messageState.round.assistantMessageId);
      // An authoritative messageAppended may arrive before roundFinished and
      // legitimately clear `_streaming`. Keep the projection round anchored
      // by message identity until its terminal event is reduced; otherwise a
      // stale terminal can bypass the shared reducer's identity guard.
      if (!activeMessage || activeMessage.role !== "assistant") {
        messageState = createChatMessageState(conversationId, options.allMessages.value);
      }
    }
    return messageState;
  }

  function synchronizeDisplayRefs(messageId: string) {
    const message = options.allMessages.value.find((item) => item.id === messageId);
    if (!message) return;
    const blocks = assistantContentBlocksFromMessage(message);
    if (options.streamBlocks) options.streamBlocks.value = blocks;
    options.latestAssistantText.value = assistantTextFromStreamBlocks(blocks) || readMessagePlainText(message);
    const meta = (message.providerMeta || {}) as Record<string, unknown>;
    options.toolStatusText.value = String(meta._toolStatusText || "");
    const toolStatusState = String(meta._toolStatusState || "").trim();
    options.toolStatusState.value = toolStatusState === "running" || toolStatusState === "done" || toolStatusState === "failed"
      ? toolStatusState
      : "";
  }

  function dispatchMessageEvent(event: ChatMessageEvent): ChatMessageState {
    synchronizeMachineInput();
    messageState = reduceChatMessageState(messageState, event);
    options.allMessages.value = messageState.messages;
    const messageId = messageState.round.assistantMessageId
      || ("assistantMessageId" in event ? String(event.assistantMessageId || "").trim() : "")
      || ("assistantMessage" in event ? String(event.assistantMessage?.id || "").trim() : "");
    if (messageId) synchronizeDisplayRefs(messageId);
    return messageState;
  }

  function ensureProjectionRound(
    messageId: string,
    statusText = "",
    phase: "waiting" | "streaming" = "streaming",
    identity?: Partial<RoundStartedPayload>,
  ) {
    const normalizedMessageId = String(messageId || "").trim();
    if (!normalizedMessageId) return;
    synchronizeMachineInput();
    if (messageState.round.phase !== "idle" && messageState.round.assistantMessageId !== normalizedMessageId) {
      dispatchMessageEvent({
        type: "round_reset",
        conversationId: machineConversationId(),
        preserveVisibleContent: true,
      });
    }
    if (messageState.round.phase === "idle") {
      const existing = options.allMessages.value.find((message) => message.id === normalizedMessageId);
      const event = tauriRoundStartedToMessageEvent({
        ...identity,
        startedAt: identity?.startedAt || existing?.createdAt || new Date().toISOString(),
      }, machineConversationId(), {
        assistantMessageId: normalizedMessageId,
        speakerAgentId: resolveAssistantMessageSpeakerAgentId(existing),
        statusText,
        phase,
      });
      if (event) dispatchMessageEvent(event);
    }
  }

  function resolveAssistantMessageSpeakerAgentId(existingMessage?: ChatMessage | null): string {
    const existing = String(existingMessage?.speakerAgentId || "").trim();
    if (existing && existing !== "assistant-draft") return existing;
    const activeRoundAgentId = String(options.getActiveRoundAgentId ? options.getActiveRoundAgentId() : "").trim();
    if (activeRoundAgentId) return activeRoundAgentId;
    return "assistant-draft";
  }

  function getPendingUserDraftId(): string {
    return pendingUserDraftId;
  }

  function getPendingUserDraftIdForGen(gen: number): string {
    return pendingUserDraftIdByGen.get(gen) || "";
  }

  function getMessageStreamBlocks(messageId: string): AssistantStreamBlock[] {
    if (!messageId) return [];
    const draft = options.allMessages.value.find((item) => item.id === messageId);
    return assistantContentBlocksFromMessage(draft);
  }

  function loadStreamBlocksFromMessage(messageId: string) {
    if (!options.streamBlocks) return;
    if (!messageId) {
      options.streamBlocks.value = [];
      return;
    }
    const draft = options.allMessages.value.find((item) => item.id === messageId);
    const blocks = assistantContentBlocksFromMessage(draft);
    if (blocks.length > 0 || options.streamBlocks.value.length === 0) {
      options.streamBlocks.value = blocks;
    }
  }

  function hasStreamingAssistantMessageInMessages(): boolean {
    return options.allMessages.value.some((message) => {
      const messageId = String(message?.id || "").trim();
      const meta = (message?.providerMeta || {}) as Record<string, unknown>;
      return messageId.startsWith(DRAFT_ASSISTANT_ID_PREFIX)
        || (String(message?.role || "").trim() === "assistant" && meta._streaming === true);
    });
  }

  function insertUserDraft(
    rawMessageId: string,
    gen: number,
    text: string,
    images: Array<{ mime: string; bytesBase64: string; savedPath?: string }>,
    attachments: Array<{ fileName: string; path: string; mime: string }>,
    extraTextBlocks: string[],
    mentions: ChatMentionTarget[],
  ): string {
    const messageId = String(rawMessageId || "").trim();
    if (!messageId) return "";
    const parts: ChatMessage["parts"] = [];
    const normalizedText = String(text || "");
    if (normalizedText) {
      parts.push({ type: "text", text: normalizedText });
    }
    const seenAttachmentPaths = new Set<string>();
    for (const image of images) {
      const mime = String(image.mime || "").trim();
      const path = String(image.savedPath || "").trim().replace(/\\/g, "/");
      if (!mime || !path) continue;
      seenAttachmentPaths.add(path.toLowerCase());
      parts.push({ type: "attachment", path, mime, name: path.split("/").pop() || "image" });
    }
    for (const attachment of attachments) {
      const path = String(attachment.path || "").trim().replace(/\\/g, "/");
      if (!path || seenAttachmentPaths.has(path.toLowerCase())) continue;
      seenAttachmentPaths.add(path.toLowerCase());
      parts.push({
        type: "attachment",
        path,
        mime: String(attachment.mime || "").trim(),
        name: String(attachment.fileName || "").trim() || path.split("/").pop() || "attachment",
      });
    }
    const msg: ChatMessage = {
      id: messageId,
      role: "user",
      createdAt: new Date().toISOString(),
      speakerAgentId: "user-persona",
      parts,
      extraTextBlocks: Array.isArray(extraTextBlocks) ? extraTextBlocks.filter((item) => !!String(item || "").trim()) : [],
      providerMeta: {
        message_meta: mentions.length > 0
          ? {
              kind: "user_message",
              mentions: mentions.map((item) => ({
                agentId: item.agentId,
                agentName: item.agentName,
                departmentId: item.departmentId,
                departmentName: item.departmentName,
              })),
            }
        : undefined,
      },
    };
    const stableMsg = messageWithStableRenderId(msg, messageId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === messageId);
    if (idx >= 0) {
      return messageId;
    }
    options.allMessages.value = [...cur, stableMsg];
    return messageId;
  }

  function insertStreamingAssistantMessage(messageId: string, gen?: number, initialText = ""): string {
    const normalizedMessageId = String(messageId || "").trim();
    if (!normalizedMessageId) return "";
    ensureProjectionRound(normalizedMessageId, initialText, "streaming");
    dispatchMessageEvent({
      type: "assistant_stream_snapshot",
      conversationId: machineConversationId(),
      assistantMessageId: normalizedMessageId,
      snapshot: {
        persistedAssistantMessageId: normalizedMessageId,
        startedAtMs: typeof gen === "number" ? options.getSendStartedAtMs(gen) || undefined : undefined,
        speakerAgentId: resolveAssistantMessageSpeakerAgentId(
          options.allMessages.value.find((message) => message.id === normalizedMessageId),
        ),
        streamBlocks: options.streamBlocks?.value || [],
        toolStatusText: options.toolStatusText.value,
        toolStatusState: options.toolStatusState.value,
        preStreamingStatusText: String(initialText || ""),
        frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    });
    return normalizedMessageId;
  }

  function updateQueuedAssistantMessageStatus(
    messageId: string,
    statusText: string,
    identity?: Partial<RoundStartedPayload>,
  ) {
    if (!messageId) return;
    const existingMessage = options.allMessages.value.find((item) => item.id === messageId);
    if (String(existingMessage?.role || "") === "assistant") {
      const existingMeta = (existingMessage?.providerMeta || {}) as Record<string, unknown>;
      if (existingMeta._streaming !== true || assistantMessageHasVisibleProgress(existingMessage)) {
        return;
      }
    }
    ensureProjectionRound(messageId, statusText, "waiting", identity);
    dispatchMessageEvent({
      type: "assistant_stream_snapshot",
      conversationId: machineConversationId(),
      assistantMessageId: messageId,
      snapshot: {
        persistedAssistantMessageId: messageId,
        speakerAgentId: resolveAssistantMessageSpeakerAgentId(existingMessage),
        streamBlocks: assistantContentBlocksFromMessage(existingMessage),
        toolStatusText: options.toolStatusText.value,
        toolStatusState: options.toolStatusState.value,
        preStreamingStatusText: String(statusText || ""),
        frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    });
  }

  function readMessageStreamSegments(messageId: string): string[] {
    if (!messageId) return [];
    const message = options.allMessages.value.find((item) => item.id === messageId);
    const meta = (message?.providerMeta || {}) as Record<string, unknown>;
    if (!Array.isArray(meta._streamSegments)) return [];
    return (meta._streamSegments as unknown[])
      .map((item) => String(item ?? ""))
      .filter((item) => item.length > 0);
  }

  function readMessageStreamTail(messageId: string): string {
    if (!messageId) return "";
    const message = options.allMessages.value.find((item) => item.id === messageId);
    const meta = (message?.providerMeta || {}) as Record<string, unknown>;
    return String(meta._streamTail ?? "");
  }

  function syncStreamBlocksToMessage(messageId: string, rawBlocks?: AssistantStreamBlock[]) {
    if (!messageId) return;
    const blocks = normalizeAssistantStreamBlocks(rawBlocks);
    ensureProjectionRound(messageId, "", "streaming");
    dispatchMessageEvent({
      type: "assistant_stream_snapshot",
      conversationId: machineConversationId(),
      assistantMessageId: messageId,
      snapshot: {
        persistedAssistantMessageId: messageId,
        streamBlocks: blocks,
        toolStatusText: options.toolStatusText.value,
        toolStatusState: options.toolStatusState.value,
        frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    });
  }

  function updateMessageText(
    messageId: string,
    streamSegments?: string[],
    streamTail?: string,
    streamAnimatedDelta = "",
    rawBlocks?: AssistantStreamBlock[],
    updateOptions?: UpdateMessageTextOptions,
  ) {
    if (!messageId) return;
    const existingMessage = options.allMessages.value.find((item) => item.id === messageId);
    const agentId = resolveAssistantMessageSpeakerAgentId(existingMessage);
    const existingMessageText = readMessagePlainText(existingMessage);
    const nextAssistantText = String(options.latestAssistantText.value || "");
    const shouldPreserveExistingMessageText =
      !!existingMessage
      && !nextAssistantText
      && !!existingMessageText
      && (
        !!String(options.toolStatusText.value || "").trim()
        || (options.streamBlocks?.value.length || 0) > 0
      );
    if (shouldPreserveExistingMessageText) {
      options.latestAssistantText.value = existingMessageText;
    }
    const nextStreamSegments = streamSegments || readMessageStreamSegments(messageId);
    const nextStreamTail = streamTail ?? readMessageStreamTail(messageId);
    const hasVisibleStreamContent =
      !!String(options.latestAssistantText.value || "").trim()
      || nextStreamSegments.some((item) => !!String(item || "").trim())
      || !!String(nextStreamTail || "").trim()
      || (options.streamBlocks?.value.length || 0) > 0;
    const preStreamingStatusText = hasVisibleStreamContent
      ? ""
      : String(options.toolStatusText.value || "").trim();
    const streamBlocks = rawBlocks === undefined
      ? getMessageStreamBlocks(messageId)
      : normalizeAssistantStreamBlocks(rawBlocks);
    ensureProjectionRound(messageId, preStreamingStatusText, "streaming");
    dispatchMessageEvent({
      type: "assistant_stream_snapshot",
      conversationId: machineConversationId(),
      assistantMessageId: messageId,
      snapshot: {
        persistedAssistantMessageId: messageId,
        assistantText: options.latestAssistantText.value,
        speakerAgentId: agentId,
        streamBlocks,
        streamSegments: nextStreamSegments,
        streamTail: nextStreamTail,
        streamAnimatedDelta: String(streamAnimatedDelta || ""),
        preStreamingStatusText,
        toolStatusText: options.toolStatusText.value,
        toolStatusState: options.toolStatusState.value,
        frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    });
    void updateOptions;
  }

  function removeMessage(messageId: string) {
    if (!messageId) return;
    const existing = options.allMessages.value.find((message) => message.id === messageId);
    // 有内容的消息禁止删除；撤回走后端截断/整表替换，不经过这里。
    if (String(existing?.role || "").trim() === "assistant") {
      if (assistantMessageHasCanonicalVisibleContent(existing)) {
        finalizeMessage(messageId);
        return;
      }
      synchronizeMachineInput();
      if (messageState.round.phase !== "idle" && messageState.round.assistantMessageId === messageId) {
        dispatchMessageEvent({
          type: "round_failed",
          conversationId: machineConversationId(),
          assistantMessageId: messageId,
        });
        return;
      }
    } else if (messageHasVisibleContent(existing)) {
      return;
    }
    if (messageId === pendingUserDraftId) {
      pendingUserDraftId = "";
    }
    for (const [gen, userDraftId] of pendingUserDraftIdByGen.entries()) {
      if (userDraftId === messageId) {
        pendingUserDraftIdByGen.delete(gen);
      }
    }
    options.allMessages.value = options.allMessages.value.filter((m) => m.id !== messageId);
  }

  function removeLegacyAssistantDrafts() {
    options.allMessages.value = options.allMessages.value.filter((message) => {
      const messageId = String(message?.id || "").trim();
      if (!messageId.startsWith(DRAFT_ASSISTANT_ID_PREFIX)) return true;
      // 旧 draft 前缀也可能已有可见内容，仍禁止删除。
      return messageHasVisibleContent(message);
    }).map((message) => {
      const messageId = String(message?.id || "").trim();
      if (!messageId.startsWith(DRAFT_ASSISTANT_ID_PREFIX) || !messageHasVisibleContent(message)) {
        return message;
      }
      // 有内容的 draft 前缀消息：只收口流式态，不删。
      return reconcileCompletedAssistantMessage(message) || message;
    });
  }

  function finalizeMessage(
    messageId: string,
    finalMessage?: ChatMessage,
    identity?: { activationId?: string; requestId?: string },
  ) {
    if (!messageId) return;
    dispatchMessageEvent({
      type: "round_finished",
      conversationId: machineConversationId(),
      assistantMessageId: messageId,
      activationId: identity?.activationId,
      requestId: identity?.requestId,
      assistantMessage: finalMessage,
    });
  }

  function failMessage(
    messageId: string,
    error?: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) {
    if (!messageId) return;
    dispatchMessageEvent({
      type: "round_failed",
      conversationId: machineConversationId(),
      assistantMessageId: messageId,
      activationId: identity?.activationId,
      requestId: identity?.requestId,
      error,
    });
  }

  function applyAssistantEventToMessage(messageId: string, parsed: AssistantDeltaEvent) {
    if (!messageId) return;
    ensureProjectionRound(messageId, "", "streaming");
    const event = tauriAssistantDeltaToMessageEvent(parsed, machineConversationId(), messageId);
    if (event) dispatchMessageEvent(event);
  }

  function applyAssistantDeltaToMessage(messageId: string, delta: string) {
    if (!messageId || !delta) return;
    applyAssistantEventToMessage(messageId, { delta });
  }

  return {
    applyAssistantDeltaToMessage,
    applyAssistantEventToMessage,
    failMessage,
    finalizeMessage,
    getMessageStreamBlocks,
    getPendingUserDraftId,
    getPendingUserDraftIdForGen,
    hasStreamingAssistantMessageInMessages,
    insertStreamingAssistantMessage,
    insertUserDraft,
    loadStreamBlocksFromMessage,
    removeLegacyAssistantDrafts,
    removeMessage,
    syncStreamBlocksToMessage,
    updateMessageText,
    updateQueuedAssistantMessageStatus,
  };
}

export function summarizeToolCallsText(streamBlocks?: AssistantStreamBlock[]): string {
  const toolCalls = streamBlocksToToolCalls(streamBlocks || []);
  if (toolCalls.length <= 0) return "";
  const lastToolName = toolCalls[toolCalls.length - 1]?.name || "";
  const extraCount = Math.max(0, toolCalls.length - 1);
  return extraCount > 0
    ? `调用 ${lastToolName || "-"} (+${extraCount})`
    : `调用 ${lastToolName || "-"}`;
}
