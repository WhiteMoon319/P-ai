import type { Ref } from "vue";
import type { AssistantStreamBlock } from "../../../types/app";
import {
  applyAssistantToolEventToStreamBlocks,
  appendReasoningDeltaToStreamBlocks,
  appendTextDeltaToStreamBlocks,
  normalizeAssistantStreamBlocks,
} from "../../../utils/chat-message-semantics";
import {
  readDeltaMessage,
  type AssistantDeltaEvent,
} from "./use-chat-flow-events";
import {
  normalizeConversationId,
  positiveRoundedNumber,
} from "./use-chat-flow-utils";

export type ConversationStreamCache = {
  activationId?: string;
  requestId?: string;
  startedAt?: string;
  startedAtMs?: number;
  frontendDispatchStartedAtMs?: number;
  frontendDispatchElapsedMs?: number;
  assistantText: string;
  toolStatusText: string;
  toolStatusState: "running" | "done" | "failed" | "";
  streamBlocks: AssistantStreamBlock[];
  persistedAssistantMessageId?: string;
};

export type ConversationRuntimeStreamCacheSnapshot = {
  activationId?: string;
  requestId?: string;
  startedAt?: string;
  startedAtMs?: number;
  frontendDispatchStartedAtMs?: number;
  frontendDispatchElapsedMs?: number;
  assistantText?: string;
  toolStatusText?: string;
  toolStatusState?: "running" | "done" | "failed" | "" | string;
  streamBlocks?: AssistantStreamBlock[];
  hasVisibleProgress?: boolean;
  persistedAssistantMessageId?: string;
};

type UseChatFlowStreamCacheOptions = {
  getConversationId?: () => string;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  getActiveActivationId: () => string;
  getFrontendDispatchStartedAtMs: () => number;
  getFrontendDispatchElapsedMs: () => number;
  currentFrontendDispatchElapsedMs: () => number;
  restoreFrontendDispatchTimerFromCache: (cache: ConversationStreamCache) => void;
};

function normalizeToolStatusState(value: unknown): "running" | "done" | "failed" | "" {
  const status = String(value || "").trim();
  return status === "running" || status === "done" || status === "failed" ? status : "";
}

export function streamCacheHasVisibleProgress(
  cache?: ConversationRuntimeStreamCacheSnapshot | ConversationStreamCache | null,
): boolean {
  if (!cache) return false;
  return !!(
    String(cache.assistantText || "").trim()
    || String(cache.toolStatusText || "").trim()
    || String(cache.toolStatusState || "").trim()
    || (Array.isArray(cache.streamBlocks) && cache.streamBlocks.length > 0)
  );
}

function emptyConversationStreamCache(): ConversationStreamCache {
  return {
      activationId: "",
      requestId: "",
      startedAt: "",
      startedAtMs: 0,
    frontendDispatchStartedAtMs: 0,
    frontendDispatchElapsedMs: 0,
    assistantText: "",
    toolStatusText: "",
    toolStatusState: "",
    streamBlocks: [],
  };
}

export function useChatFlowStreamCache(options: UseChatFlowStreamCacheOptions) {
  const conversationStreamCache = new Map<string, ConversationStreamCache>();

  function readConversationStreamCache(conversationId?: string | null): ConversationStreamCache | null {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return null;
    const cache = conversationStreamCache.get(cid);
    if (!cache) return null;
    return {
      activationId: String(cache.activationId || "").trim(),
      requestId: String(cache.requestId || "").trim(),
      startedAt: String(cache.startedAt || "").trim(),
      startedAtMs: positiveRoundedNumber(cache.startedAtMs),
      frontendDispatchStartedAtMs: positiveRoundedNumber(cache.frontendDispatchStartedAtMs),
      frontendDispatchElapsedMs: positiveRoundedNumber(cache.frontendDispatchElapsedMs),
      assistantText: cache.assistantText,
      toolStatusText: cache.toolStatusText,
      toolStatusState: cache.toolStatusState,
      streamBlocks: normalizeAssistantStreamBlocks(cache.streamBlocks),
    };
  }

  function writeConversationStreamCache(
    conversationId: string,
    updater: (current: ConversationStreamCache) => ConversationStreamCache,
  ) {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return;
    const next = updater(readConversationStreamCache(cid) || emptyConversationStreamCache());
    conversationStreamCache.set(cid, {
      ...next,
      activationId: String(next.activationId || "").trim(),
      requestId: String(next.requestId || "").trim(),
      startedAt: String(next.startedAt || "").trim(),
      startedAtMs: positiveRoundedNumber(next.startedAtMs),
      frontendDispatchStartedAtMs: positiveRoundedNumber(next.frontendDispatchStartedAtMs),
      frontendDispatchElapsedMs: positiveRoundedNumber(next.frontendDispatchElapsedMs),
      streamBlocks: normalizeAssistantStreamBlocks(next.streamBlocks),
    });
  }

  function clearConversationStreamCache(conversationId?: string | null) {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return;
    conversationStreamCache.delete(cid);
  }

  function syncCurrentDisplayStateToConversationStreamCache(conversationId?: string | null) {
    const cid = normalizeConversationId(conversationId || (options.getConversationId ? options.getConversationId() : ""));
    if (!cid) return;
    const activeActivationId = options.getActiveActivationId();
    writeConversationStreamCache(cid, (current) => ({
      assistantText: String(options.latestAssistantText.value || ""),
      activationId: activeActivationId,
      requestId: activeActivationId,
      startedAt: current.startedAt,
      startedAtMs: current.startedAtMs,
      frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
      frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      toolStatusText: String(options.toolStatusText.value || ""),
      toolStatusState: options.toolStatusState.value,
      streamBlocks: Array.isArray(options.streamBlocks?.value)
        ? normalizeAssistantStreamBlocks(options.streamBlocks.value)
        : normalizeAssistantStreamBlocks(current.streamBlocks),
    }));
  }

  function applyConversationStreamCacheToDisplay(
    conversationId?: string | null,
    input?: { ignoreActivationId?: boolean; skipStreamBlocks?: boolean },
  ): boolean {
    const cache = readConversationStreamCache(conversationId);
    if (!cache) return false;
    const activeActivationId = options.getActiveActivationId();
    if (!input?.ignoreActivationId && activeActivationId && cache.activationId && cache.activationId !== activeActivationId) {
      return false;
    }
    if (cache.assistantText || !options.latestAssistantText.value) {
      options.latestAssistantText.value = cache.assistantText;
    }
    options.restoreFrontendDispatchTimerFromCache(cache);
    if (cache.toolStatusText || !options.toolStatusText.value) {
      options.toolStatusText.value = cache.toolStatusText;
    }
    if (cache.toolStatusState || !options.toolStatusState.value) {
      options.toolStatusState.value = cache.toolStatusState;
    }
    if (options.streamBlocks && !input?.skipStreamBlocks) {
      if (cache.streamBlocks.length > 0 || options.streamBlocks.value.length === 0) {
        options.streamBlocks.value = normalizeAssistantStreamBlocks(cache.streamBlocks);
      }
    }
    return true;
  }

  function writeConversationStreamCacheSnapshot(
    conversationId: string,
    snapshot?: ConversationRuntimeStreamCacheSnapshot | null,
  ) {
    const cid = normalizeConversationId(conversationId);
    if (!cid || !snapshot) return;
    writeConversationStreamCache(cid, (current) => ({
      activationId: String(snapshot.activationId || snapshot.requestId || current.activationId || "").trim(),
      requestId: String(snapshot.requestId || snapshot.activationId || current.requestId || "").trim(),
      startedAt: String(snapshot.startedAt || current.startedAt || "").trim(),
      startedAtMs: positiveRoundedNumber(snapshot.startedAtMs || current.startedAtMs),
      frontendDispatchStartedAtMs: positiveRoundedNumber(snapshot.startedAtMs || snapshot.frontendDispatchStartedAtMs || current.frontendDispatchStartedAtMs),
      frontendDispatchElapsedMs: positiveRoundedNumber(snapshot.frontendDispatchElapsedMs || current.frontendDispatchElapsedMs),
      assistantText: String(snapshot.assistantText || ""),
      toolStatusText: String(snapshot.toolStatusText || ""),
      toolStatusState: normalizeToolStatusState(snapshot.toolStatusState),
      streamBlocks: normalizeAssistantStreamBlocks(snapshot.streamBlocks).length > 0
        ? normalizeAssistantStreamBlocks(snapshot.streamBlocks)
        : normalizeAssistantStreamBlocks(current.streamBlocks),
      persistedAssistantMessageId: String(snapshot.persistedAssistantMessageId || current.persistedAssistantMessageId || "").trim(),
    }));
  }

  function applyConversationStreamCacheSnapshotToDisplay(
    conversationId: string,
    snapshot?: ConversationRuntimeStreamCacheSnapshot | null,
    input?: { ignoreActivationId?: boolean },
  ): boolean {
    writeConversationStreamCacheSnapshot(conversationId, snapshot);
    return applyConversationStreamCacheToDisplay(conversationId, input);
  }

  function applyAssistantEventToConversationStreamCache(
    conversationId: string,
    parsed: AssistantDeltaEvent,
  ): boolean {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return false;
    let changed = false;
    writeConversationStreamCache(cid, (current) => {
      const activeActivationId = options.getActiveActivationId();
      const next: ConversationStreamCache = {
        ...current,
        activationId: String(parsed.activationId || parsed.requestId || current.activationId || activeActivationId || "").trim(),
        requestId: String(parsed.requestId || parsed.activationId || current.requestId || activeActivationId || "").trim(),
        startedAt: current.startedAt,
        startedAtMs: current.startedAtMs,
        frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
        streamBlocks: normalizeAssistantStreamBlocks(current.streamBlocks),
      };
      const delta = readDeltaMessage(parsed);
      if (parsed.kind === "tool_status") {
        next.toolStatusText = parsed.message || "";
        next.toolStatusState =
          parsed.toolStatus === "running" || parsed.toolStatus === "done" || parsed.toolStatus === "failed"
            ? parsed.toolStatus : "";
        changed = true;
        return next;
      }
      if (parsed.kind === "assistant_tool_event") {
        next.streamBlocks = applyAssistantToolEventToStreamBlocks(next.streamBlocks, parsed.message);
        changed = true;
        return next;
      }
      if (parsed.kind === "activity_reasoning_delta" && delta) {
        next.streamBlocks = appendReasoningDeltaToStreamBlocks(next.streamBlocks, delta);
        changed = true;
        return next;
      }
      if (delta) {
        next.assistantText += delta;
        next.streamBlocks = appendTextDeltaToStreamBlocks(next.streamBlocks, delta);
        changed = true;
      }
      return next;
    });
    return changed;
  }

  return {
    applyAssistantEventToConversationStreamCache,
    applyConversationStreamCacheSnapshotToDisplay,
    applyConversationStreamCacheToDisplay,
    clearConversationStreamCache,
    readConversationStreamCache,
    syncCurrentDisplayStateToConversationStreamCache,
    writeConversationStreamCacheSnapshot,
  };
}
