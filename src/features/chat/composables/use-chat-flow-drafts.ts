import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMentionTarget, ChatMessage } from "../../../types/app";
import {
  assistantTextFromStreamBlocks,
  assistantContentBlocksFromMessage,
  appendTextDeltaToStreamBlocks,
  normalizeAssistantStreamBlocks,
  normalizeChatActivityItems,
  streamBlocksToToolCalls,
} from "../../../utils/chat-message-semantics";
import { consumeClosedMarkdownBlocks } from "./use-chat-flow-text";
import { readMessagePlainText, messageHasVisibleContent } from "./use-chat-flow-utils";
import { messageWithStableRenderId, stableRenderIdFromMessage } from "../utils/stable-render-id";

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
    const startedAtMs = typeof gen === "number" ? options.getSendStartedAtMs(gen) || 0 : 0;
    const elapsedMs = startedAtMs > 0 ? Math.max(0, Date.now() - startedAtMs) : -1;
    const agentId = resolveAssistantMessageSpeakerAgentId();
    const msg = messageWithStableRenderId({
      id: normalizedMessageId,
      role: "assistant",
      createdAt: new Date().toISOString(),
      speakerAgentId: agentId,
      parts: [{ type: "text", text: "" }],
      providerMeta: {
        _streaming: true,
        _streamSegments: [] as string[],
        _streamTail: "",
        _preStreamingStatusText: String(initialText || ""),
        _toolStatusText: String(options.toolStatusText.value || ""),
        _toolStatusState: "",
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    } satisfies ChatMessage, normalizedMessageId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === normalizedMessageId);
    if (idx >= 0) {
      const existing = cur[idx];
      const existingMeta = (existing?.providerMeta || {}) as Record<string, unknown>;
      if (String(existing?.role || "") === "assistant" && (existingMeta._streaming !== true || assistantMessageHasVisibleProgress(existing))) {
        return normalizedMessageId;
      }
      options.allMessages.value = cur.map((m, i) => (i === idx ? msg : m));
      return normalizedMessageId;
    }
    options.allMessages.value = [...cur, msg];
    return normalizedMessageId;
  }

  function updateQueuedAssistantMessageStatus(messageId: string, statusText: string) {
    if (!messageId) return;
    const existingMessage = options.allMessages.value.find((item) => item.id === messageId);
    if (String(existingMessage?.role || "") === "assistant") {
      const existingMeta = (existingMessage?.providerMeta || {}) as Record<string, unknown>;
      if (existingMeta._streaming !== true || assistantMessageHasVisibleProgress(existingMessage)) {
        return;
      }
    }
    const agentId = resolveAssistantMessageSpeakerAgentId(existingMessage);
    const existingMeta = ((existingMessage?.providerMeta || {}) as Record<string, unknown>);
    const stableRenderId = stableRenderIdFromMessage(existingMessage) || messageId;
    const msg = messageWithStableRenderId({
      id: messageId,
      role: "assistant",
      createdAt: String(existingMessage?.createdAt || new Date().toISOString()),
      speakerAgentId: agentId,
      parts: [{ type: "text", text: "" }],
      providerMeta: {
        ...existingMeta,
        _streaming: true,
        _streamSegments: [] as string[],
        _streamTail: "",
        _streamAnimatedDelta: "",
        _preStreamingStatusText: String(statusText || ""),
        _toolStatusText: String(options.toolStatusText.value || ""),
        _toolStatusState: String(existingMeta._toolStatusState || ""),
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    } satisfies ChatMessage, stableRenderId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === messageId);
    if (idx >= 0) {
      options.allMessages.value = cur.map((m, i) => (i === idx ? msg : m));
    } else {
      options.allMessages.value = [...cur, msg];
    }
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
    options.allMessages.value = options.allMessages.value.map((message) => {
      if (message.id !== messageId) return message;
      const meta = ((message.providerMeta || {}) as Record<string, unknown>);
      return {
        ...message,
        contentBlocks: blocks,
        providerMeta: meta,
      };
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
    const stableRenderId = stableRenderIdFromMessage(existingMessage) || messageId;
    const existingMeta = ((existingMessage?.providerMeta || {}) as Record<string, unknown>);
    const msg = messageWithStableRenderId({
      id: messageId,
      role: "assistant",
      createdAt: String(existingMessage?.createdAt || new Date().toISOString()),
      speakerAgentId: agentId,
      parts: existingMessage?.parts || [{ type: "text", text: "" }],
      contentBlocks: streamBlocks,
      toolCall: existingMessage?.toolCall,
      activityItems: existingMessage?.activityItems,
      providerMeta: {
        ...existingMeta,
        _streaming: true,
        _streamSegments: nextStreamSegments,
        _streamTail: nextStreamTail,
        _streamAnimatedDelta: String(streamAnimatedDelta || ""),
        _preStreamingStatusText: preStreamingStatusText,
        _toolStatusText: String(options.toolStatusText.value || ""),
        _toolStatusState: String(options.toolStatusState.value || ""),
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    } satisfies ChatMessage, stableRenderId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === messageId);
    options.allMessages.value = idx < 0 ? [...cur, msg] : cur.map((m, i) => (i === idx ? msg : m));
  }

  function removeMessage(messageId: string) {
    if (!messageId) return;
    const existing = options.allMessages.value.find((message) => message.id === messageId);
    // 有内容的消息禁止删除；撤回走后端截断/整表替换，不经过这里。
    if (messageHasVisibleContent(existing)) {
      if (String(existing?.role || "").trim() === "assistant") {
        finalizeMessage(messageId);
      }
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
      const meta = { ...((message.providerMeta || {}) as Record<string, unknown>) };
      delete meta._streaming;
      delete meta._preStreamingStatusText;
      delete meta._toolStatusText;
      delete meta._toolStatusState;
      return {
        ...message,
        providerMeta: meta,
      };
    });
  }

  function finalizeMessage(messageId: string, finalMessage?: ChatMessage) {
    if (!messageId) return;
    const current = options.allMessages.value;
    const messageIdx = current.findIndex((m) => m.id === messageId);
    if (messageIdx < 0) return;
    const draft = current[messageIdx];
    const stableRenderId = stableRenderIdFromMessage(draft) || messageId;
    const draftMeta = ((draft.providerMeta || {}) as Record<string, unknown>);
    const finalMeta = ((finalMessage?.providerMeta || {}) as Record<string, unknown>);
    const speakerAgentId = resolveAssistantMessageSpeakerAgentId(draft);
    const canonicalBlocks = getMessageStreamBlocks(messageId);
    const hasCanonicalContent = canonicalBlocks.length > 0;

    // 完成态只收口流式状态，不整条替换气泡身份。
    // 保留原 messageId / _stableRenderId，避免 virtual list key 变化导致跳位。
    const nextMeta: Record<string, unknown> = { ...finalMeta, ...draftMeta };
    delete nextMeta._streaming;
    delete nextMeta._preStreamingStatusText;
    delete nextMeta._toolStatusText;
    delete nextMeta._toolStatusState;

    const finalNonTextParts = Array.isArray(finalMessage?.parts)
      ? finalMessage.parts.filter((part) => part?.type !== "text")
      : [];
    const draftTextParts = Array.isArray(draft.parts)
      ? draft.parts.filter((part) => part?.type === "text")
      : [];
    const completedBase = !hasCanonicalContent && finalMessage
      ? finalMessage
      : draft;
    const completedFields = hasCanonicalContent && finalMessage
      ? { ...draft, ...finalMessage }
      : completedBase;
    const normalized = messageWithStableRenderId({
      ...completedFields,
      id: messageId,
      role: completedBase.role,
      createdAt: draft.createdAt || completedBase.createdAt,
      speakerAgentId,
      parts: hasCanonicalContent
        ? [...draftTextParts, ...finalNonTextParts]
        : completedBase.parts,
      contentBlocks: hasCanonicalContent ? canonicalBlocks : completedBase.contentBlocks,
      toolCall: hasCanonicalContent ? draft.toolCall : completedBase.toolCall,
      activityItems: hasCanonicalContent ? draft.activityItems : completedBase.activityItems,
      providerMeta: nextMeta,
    } satisfies ChatMessage, stableRenderId);

    const finalId = String(finalMessage?.id || "").trim();
    const nextMessages = finalId && finalId !== messageId
      ? current.filter((message, index) => index === messageIdx || message.id !== finalId)
      : current;
    const nextMessageIdx = nextMessages.findIndex((message) => message.id === messageId);
    if (nextMessageIdx < 0) return;
    options.allMessages.value = nextMessages.map((message, index) => (
      index === nextMessageIdx ? normalized : message
    ));
  }

  function applyAssistantDeltaToMessage(messageId: string, delta: string) {
    if (!messageId || !delta) return;
    options.latestAssistantText.value += delta;
    const blocks = appendTextDeltaToStreamBlocks(getMessageStreamBlocks(messageId), delta);
    syncStreamBlocksToMessage(messageId, blocks);
    const currentSegments = readMessageStreamSegments(messageId);
    const currentTail = readMessageStreamTail(messageId);
    const parsed = consumeClosedMarkdownBlocks(`${currentTail}${delta}`);
    const nextStreamSegments = parsed.chunks.length > 0
      ? [...currentSegments, ...parsed.chunks]
      : currentSegments;
    updateMessageText(messageId, nextStreamSegments, parsed.tail, delta, blocks, {
      preserveActivityProjection: true,
    });
  }

  return {
    applyAssistantDeltaToMessage,
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
