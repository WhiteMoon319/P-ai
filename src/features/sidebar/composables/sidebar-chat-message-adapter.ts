import type { ChatMessage } from "../../../types/app";
import type {
  ChatAssistantDelta,
  ChatMessageEvent,
} from "../../chat/composables/chat-message-state-machine";
import type { SidebarAssistantDeltaPayload } from "../sidebar-app-types";

function normalized(value: unknown): string {
  return String(value || "").trim();
}

export function sidebarRoundStartedToMessageEvent(
  payload: unknown,
  fallbackConversationId: string,
  input?: { startedAt?: string; speakerAgentId?: string },
): Extract<ChatMessageEvent, { type: "round_started" }> | null {
  const value = payload && typeof payload === "object"
    ? payload as Record<string, unknown>
    : {};
  const conversationId = normalized(value.conversationId || fallbackConversationId);
  const assistantMessageId = normalized(value.assistantMessageId);
  if (!conversationId || !assistantMessageId) return null;
  return {
    type: "round_started",
    conversationId,
    assistantMessageId,
    activationId: normalized(value.activationId) || undefined,
    requestId: normalized(value.requestId) || undefined,
    revision: normalized(value.updatedAt) || undefined,
    startedAt: normalized(value.startedAt || input?.startedAt) || undefined,
    startedAtMs: Math.max(0, Math.round(Number(value.startedAtMs) || 0)) || undefined,
    speakerAgentId: normalized(value.agentId || input?.speakerAgentId) || undefined,
    phase: "waiting",
  };
}

export function sidebarAssistantDeltaToMessageEvent(
  payload: SidebarAssistantDeltaPayload,
  fallbackConversationId: string,
  fallbackAssistantMessageId = "",
): Extract<ChatMessageEvent, { type: "assistant_delta" }> | null {
  const conversationId = normalized(payload?.conversationId || fallbackConversationId);
  const raw = payload?.event;
  if (!conversationId || !raw) return null;
  const streamCache = raw.streamCache;
  const event: ChatAssistantDelta = {
    assistantMessageId: normalized(
      streamCache?.persistedAssistantMessageId || fallbackAssistantMessageId,
    ) || undefined,
    activationId: normalized(raw.activationId || streamCache?.activationId) || undefined,
    requestId: normalized(raw.requestId || streamCache?.requestId) || undefined,
    revision: normalized(streamCache?.updatedAt) || undefined,
    kind: normalized(raw.kind) || undefined,
    delta: typeof raw.delta === "string" ? raw.delta : undefined,
    message: typeof raw.message === "string" ? raw.message : undefined,
    toolStatus: normalized(raw.toolStatus) || undefined,
    streamCache: streamCache ? {
      activationId: normalized(streamCache.activationId) || undefined,
      requestId: normalized(streamCache.requestId) || undefined,
      updatedAt: normalized(streamCache.updatedAt) || undefined,
      assistantText: typeof streamCache.assistantText === "string" ? streamCache.assistantText : undefined,
      toolStatusText: typeof streamCache.toolStatusText === "string" ? streamCache.toolStatusText : undefined,
      toolStatusState: normalized(streamCache.toolStatusState) || undefined,
      streamBlocks: streamCache.streamBlocks,
      persistedAssistantMessageId: normalized(streamCache.persistedAssistantMessageId) || undefined,
    } : undefined,
  };
  return { type: "assistant_delta", conversationId, event };
}

export function sidebarRoundFinishedToMessageEvent(
  payload: unknown,
  fallbackConversationId: string,
  fallbackAssistantMessageId = "",
): {
  event: Extract<ChatMessageEvent, { type: "round_finished" | "round_failed" }>;
  failed: boolean;
  error?: unknown;
  assistantMessage?: ChatMessage;
} | null {
  const value = payload && typeof payload === "object"
    ? payload as Record<string, unknown>
    : {};
  const conversationId = normalized(value.conversationId || fallbackConversationId);
  if (!conversationId) return null;
  const assistantMessage = value.assistantMessage && typeof value.assistantMessage === "object"
    ? value.assistantMessage as ChatMessage
    : undefined;
  const assistantMessageId = normalized(
    assistantMessage?.id || value.assistantMessageId || fallbackAssistantMessageId,
  );
  const failed = normalized(value.status) === "failed";
  if (failed) {
    return {
      failed,
      error: value.error,
      assistantMessage,
      event: {
        type: "round_failed",
        conversationId,
        assistantMessageId: assistantMessageId || undefined,
        activationId: normalized(value.activationId) || undefined,
        requestId: normalized(value.requestId) || undefined,
        error: value.error,
      },
    };
  }
  return {
    failed,
    assistantMessage,
    event: {
      type: "round_finished",
      conversationId,
      assistantMessageId: assistantMessageId || undefined,
      activationId: normalized(value.activationId) || undefined,
      requestId: normalized(value.requestId) || undefined,
      assistantMessage,
    },
  };
}
