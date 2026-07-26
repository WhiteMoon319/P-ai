import type { Ref } from "vue";
import { normalizeAssistantStreamBlocks } from "../../../utils/chat-message-semantics";
import {
  assistantEventHasVisibleProgress,
  readDeltaMessage,
  readContextUsageUpdatePayload,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  type AssistantDeltaEvent,
  type ContextUsageUpdatePayload,
} from "./use-chat-flow-events";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";
import type { PendingTerminalEvent, RoundState } from "./use-chat-flow-types";

type UseChatFlowStreamingEventsOptions = {
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  contextUsagePreview?: Ref<ContextUsageUpdatePayload | null>;
  reasoningStartedAtMs: Ref<number>;
  getRound: () => RoundState;
  promoteQueuedRoundToStreaming: (gen: number) => number;
  setPendingTerminalEvent: (event: PendingTerminalEvent | null) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  getConversationId?: () => string;
  getActiveActivationId: () => string;
  setActiveActivationId: (value: string) => void;
  applyConversationStreamCacheSnapshotToDisplay: (
    conversationId: string,
    snapshot: ConversationRuntimeStreamCacheSnapshot,
  ) => boolean;
  handleRoundCompleted: (
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: any;
      activationId?: string;
      requestId?: string;
    },
  ) => Promise<void>;
  handleRoundFailed: (
    gen: number,
    error: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) => Promise<void>;
  applyAssistantEventToMessage: (messageId: string, parsed: AssistantDeltaEvent) => void;
  enqueueStreamDelta: (gen: number, delta: string) => void;
};

export function streamingTerminalTargetsRound(
  round: RoundState,
  activeActivationId: string,
  input: { activationId?: string; requestId?: string; assistantMessageId?: string },
): boolean {
  if (round.phase !== "queued" && round.phase !== "streaming") return false;
  const incomingMessageId = String(input.assistantMessageId || "").trim();
  if (incomingMessageId && incomingMessageId !== round.messageId) return false;
  const currentActivationId = String(activeActivationId || "").trim();
  const incomingIds = [String(input.activationId || "").trim(), String(input.requestId || "").trim()]
    .filter(Boolean);
  if (currentActivationId && incomingIds.length > 0 && !incomingIds.includes(currentActivationId)) return false;
  return true;
}

export function useChatFlowStreamingEvents(options: UseChatFlowStreamingEventsOptions) {
  function handleStreamingEvent(currentGen: number, parsed: AssistantDeltaEvent) {
    if (parsed.kind === "context_usage_update") {
      const p = readContextUsageUpdatePayload(parsed.message);
      const activeConversationId = options.getConversationId ? options.getConversationId() : "";
      if (p && (!activeConversationId || p.conversationId === activeConversationId)) {
        if (options.contextUsagePreview) {
          options.contextUsagePreview.value = p;
        }
      }
      return;
    }
    if (!currentGen) {
      return;
    }
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === currentGen && assistantEventHasVisibleProgress(parsed)) {
      options.promoteQueuedRoundToStreaming(currentGen);
    }
    const currentRound = options.getRound();
    if (currentRound.phase !== "streaming" && currentRound.phase !== "queued") {
      return;
    }
    if (currentRound.gen !== currentGen) {
      return;
    }
    if (parsed.kind === "round_completed") {
      const p = readRoundCompletedPayload(parsed.message);
      const identity = {
        activationId: p?.activationId || parsed.activationId,
        requestId: p?.requestId || parsed.requestId,
        assistantMessageId: p?.assistantMessage?.id,
      };
      if (!streamingTerminalTargetsRound(
        currentRound,
        options.getActiveActivationId(),
        identity,
      )) return;
      const result = {
        assistantText: String(p?.assistantText || ""),
        assistantMessage: p?.assistantMessage,
        activationId: identity.activationId,
        requestId: identity.requestId,
      };
      if (currentRound.phase === "queued" && parsed.reason === "context_compaction_boundary") {
        void options.handleRoundCompleted(currentGen, result);
        return;
      }
      if (currentRound.phase === "queued") {
        options.setPendingTerminalEvent({
          kind: "completed",
          gen: currentGen,
          result,
        });
        options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
        options.setActiveActivationId("");
        return;
      }
      void options.handleRoundCompleted(currentGen, result);
      return;
    }

    if (parsed.kind === "round_failed") {
      const p = readRoundFailedPayload(parsed.message);
      const identity = {
        activationId: p?.activationId || parsed.activationId,
        requestId: p?.requestId || parsed.requestId,
      };
      if (!streamingTerminalTargetsRound(
        currentRound,
        options.getActiveActivationId(),
        identity,
      )) return;
      if (options.contextUsagePreview) {
        options.contextUsagePreview.value = null;
      }
      const error = p?.error || parsed.message || JSON.stringify(parsed);
      if (currentRound.phase === "queued") {
        options.setPendingTerminalEvent({
          kind: "failed",
          gen: currentGen,
          error,
          activationId: identity.activationId,
          requestId: identity.requestId,
        });
        options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
        options.setActiveActivationId("");
        return;
      }
      void options.handleRoundFailed(currentGen, error, {
        activationId: identity.activationId,
        requestId: identity.requestId,
      });
      return;
    }

    const conversationId = options.getConversationId ? options.getConversationId() : "";
    const delta = readDeltaMessage(parsed);
    const isActivityProjectionEvent =
      parsed.kind === "activity_reasoning_delta"
      || parsed.kind === "assistant_tool_event"
      || parsed.kind === "assistant_tool_result";
    let receivedCanonicalSnapshot = false;
    if (conversationId && parsed.streamCache) {
      const streamCacheMessageId = String(parsed.streamCache.persistedAssistantMessageId || "").trim();
      if (streamCacheMessageId && currentRound.messageId && streamCacheMessageId !== currentRound.messageId) {
        return;
      }
      const snapshotBlocks = normalizeAssistantStreamBlocks(parsed.streamCache.streamBlocks);
      const snapshotHasVisibleProgress = !!(
        String(parsed.streamCache.assistantText || "").trim()
        || String(parsed.streamCache.toolStatusText || "").trim()
        || String(parsed.streamCache.toolStatusState || "").trim()
        || snapshotBlocks.length > 0
      );
      if (currentRound.phase === "streaming" && snapshotHasVisibleProgress) {
        options.applyConversationStreamCacheSnapshotToDisplay(conversationId, parsed.streamCache);
        options.applyAssistantEventToMessage(currentRound.messageId, parsed);
        receivedCanonicalSnapshot = true;
      }
    }

    if (parsed.kind === "tool_status") {
      options.toolStatusText.value = parsed.message || "";
      options.toolStatusState.value =
        parsed.toolStatus === "running" || parsed.toolStatus === "done" || parsed.toolStatus === "failed"
          ? parsed.toolStatus : "";
      if (currentRound.phase === "streaming" && !receivedCanonicalSnapshot) {
        options.applyAssistantEventToMessage(currentRound.messageId, parsed);
      }
    }

    if (isActivityProjectionEvent) {
      if (delta && options.reasoningStartedAtMs.value === 0) options.reasoningStartedAtMs.value = Date.now();
      if (currentRound.phase === "streaming" && !receivedCanonicalSnapshot) {
        options.applyAssistantEventToMessage(currentRound.messageId, parsed);
      }
    }

    if (parsed.kind === "tool_status" || isActivityProjectionEvent || receivedCanonicalSnapshot) {
      return;
    }

    options.enqueueStreamDelta(currentGen, delta);
  }

  return {
    handleStreamingEvent,
  };
}
