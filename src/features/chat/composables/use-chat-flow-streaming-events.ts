import type { Ref } from "vue";
import type { AssistantStreamBlock } from "../../../types/app";
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
    },
  ) => Promise<void>;
  handleRoundFailed: (gen: number, error: unknown) => Promise<void>;
  getMessageStreamBlocks: (messageId: string) => AssistantStreamBlock[];
  syncStreamBlocksToMessage: (messageId: string, rawBlocks?: AssistantStreamBlock[]) => void;
  updateMessageText: (
    messageId: string,
    streamSegments?: string[],
    streamTail?: string,
    streamAnimatedDelta?: string,
    rawBlocks?: AssistantStreamBlock[],
    updateOptions?: { preserveActivityProjection?: boolean },
  ) => void;
  enqueueStreamDelta: (gen: number, delta: string) => void;
};

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
      const result = {
        assistantText: String(p?.assistantText || ""),
        assistantMessage: p?.assistantMessage,
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
      if (options.contextUsagePreview) {
        options.contextUsagePreview.value = null;
      }
      const p = readRoundFailedPayload(parsed.message);
      const error = p?.error || parsed.message || JSON.stringify(parsed);
      if (currentRound.phase === "queued") {
        options.setPendingTerminalEvent({
          kind: "failed",
          gen: currentGen,
          error,
        });
        options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
        options.setActiveActivationId("");
        return;
      }
      void options.handleRoundFailed(currentGen, error);
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
        options.syncStreamBlocksToMessage(currentRound.messageId, snapshotBlocks);
        options.updateMessageText(
          currentRound.messageId,
          undefined,
          undefined,
          "",
          snapshotBlocks,
          { preserveActivityProjection: true },
        );
        receivedCanonicalSnapshot = true;
      }
    }

    if (parsed.kind === "tool_status") {
      options.toolStatusText.value = parsed.message || "";
      options.toolStatusState.value =
        parsed.toolStatus === "running" || parsed.toolStatus === "done" || parsed.toolStatus === "failed"
          ? parsed.toolStatus : "";
      if (currentRound.phase === "streaming") {
        options.updateMessageText(currentRound.messageId);
      }
    }

    if (isActivityProjectionEvent) {
      if (delta && options.reasoningStartedAtMs.value === 0) options.reasoningStartedAtMs.value = Date.now();
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
