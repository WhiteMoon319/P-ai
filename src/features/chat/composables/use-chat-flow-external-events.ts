import {
  assistantEventHasVisibleProgress,
  readAssistantEvent,
  readDeltaMessage,
  readHistoryFlushedPayload,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  readRoundStartedPayload,
} from "./use-chat-flow-events";
import type { RoundState } from "./use-chat-flow-types";
import { stringifyExternalEventPayload } from "./use-chat-flow-utils";

type UseChatFlowExternalEventsOptions = {
  debug?: boolean;
  getCurrentConversationId: () => string;
  setActiveActivationId: (value: string) => void;
  clearRecentlyCompletedRoundIds: () => void;
  hasRecentlyCompletedRoundIds: () => boolean;
  markRecentlyCompletedRoundIds: (payload: { activationId?: string; requestId?: string } | null | undefined) => void;
  matchesRecentlyCompletedRoundIds: (payload: { activationId?: string; requestId?: string } | null | undefined) => boolean;
  getRound: () => RoundState;
  getSendChatActiveGen: () => number;
  nextGeneration: () => number;
  channelBinding: {
    bindActiveConversationStream: (conversationId: string, force?: boolean) => Promise<void>;
    hasActiveBoundDeltaChannel: (conversationId?: string | null) => boolean;
    setBoundDisplayGeneration: (gen: number) => void;
  };
  handleHistoryFlushed: (gen: number, parsed: any, source: "sendChat" | "bound") => Promise<void>;
  beginAssistantActivationFromEvent: (payload: any) => number;
  markRoundStarted: (gen: number) => Promise<void>;
  handleRoundCompleted: (gen: number, result: any) => void;
  handleRoundFailed: (gen: number, error: unknown) => Promise<void>;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearFrontendDispatchTimer: () => void;
  onReloadMessages: () => Promise<void>;
  onAssistantMessageCompleted?: (input: { conversationId: string; assistantMessage: any }) => Promise<void> | void;
  setChatErrorText: (text: string, conversationId?: string | null) => void;
  formatRequestFailed: (error: unknown) => string;
  latestAssistantText: { value: string };
  chatting: { value: boolean };
  reasoningStartedAtMs: { value: number };
  applyAssistantEventToConversationStreamCache: (conversationId: string, parsed: any) => boolean;
  writeConversationStreamCacheSnapshot: (conversationId: string, snapshot?: any) => void;
  applyConversationStreamCacheToDisplay: (
    conversationId?: string | null,
    input?: { ignoreActivationId?: boolean; skipStreamBlocks?: boolean },
  ) => boolean;
  hasAssistantDraftInMessages: () => boolean;
  ensureForegroundStreamingRound: () => number;
  handleStreamingEvent: (gen: number, parsed: any) => void;
  syncStreamBlocksToDraft: (draftId: string) => void;
  updateDraftText: (draftId: string) => void;
};

export function useChatFlowExternalEvents(options: UseChatFlowExternalEventsOptions) {
  async function handleExternalStreamRebindRequired(payload: unknown) {
    void payload;
  }

  async function handleExternalHistoryFlushed(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "history_flushed");
    const parsed = readHistoryFlushedPayload(raw);
    if (!parsed) return;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return;
    }
    if (parsed.activateAssistant) {
      options.clearRecentlyCompletedRoundIds();
    }
    const treatAsSendChat = options.getSendChatActiveGen() > 0 && !!parsed.activateAssistant;
    const source: "sendChat" | "bound" = treatAsSendChat ? "sendChat" : "bound";
    const gen = treatAsSendChat ? options.getSendChatActiveGen() : options.nextGeneration();
    await options.handleHistoryFlushed(
      gen,
      {
        kind: "history_flushed",
        message: JSON.stringify(parsed),
      },
      source,
    );
  }

  async function handleExternalRoundStarted(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_started");
    const parsed = readRoundStartedPayload(raw);
    if (!parsed) return;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return;
    }
    options.clearRecentlyCompletedRoundIds();
    const gen = options.beginAssistantActivationFromEvent(parsed);
    if (!gen) return;
    await options.markRoundStarted(gen);
  }

  async function handleExternalRoundCompleted(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_completed");
    const parsed = readRoundCompletedPayload(raw);
    if (!parsed) return;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      options.clearConversationStreamCache(payloadConversationId);
      return;
    }
    options.markRecentlyCompletedRoundIds(parsed);
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      // 这里是外部终态兜底：当前前台已经不持有该轮次时，仍需走既有对账链路，避免切会话后失去正式历史刷新。
      await options.onReloadMessages();
      return;
    }
    options.handleRoundCompleted(round.gen, {
      assistantText: String(parsed.assistantText || ""),
      assistantMessage: parsed.assistantMessage,
    });
  }

  async function handleExternalRoundFailed(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_failed");
    const parsed = readRoundFailedPayload(raw);
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed?.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      const errorDetail = parsed?.error || raw || String(raw);
      options.setChatErrorText(options.formatRequestFailed(errorDetail), payloadConversationId);
      options.clearConversationStreamCache(payloadConversationId);
      return;
    }
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.latestAssistantText.value = "";
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      const errorDetail = parsed?.error || raw || String(raw);
      const errorObj = typeof errorDetail === "string" ? (
        (() => {
          try {
            return JSON.parse(errorDetail);
          } catch {
            return { message: errorDetail };
          }
        })()
      ) : errorDetail;
      console.error("[聊天流程] 非流式轮次失败", {
        roundPhase: round.phase,
        roundGen: null,
        error: errorObj,
        rawPayload: raw,
      });
      options.setChatErrorText(options.formatRequestFailed(errorDetail), payloadConversationId || currentConversationId);
      // 同上：当前前台已不承接该轮次时，保留原有外部失败后的兜底刷新。
      await options.onReloadMessages();
      return;
    }
    await options.handleRoundFailed(round.gen, parsed?.error || raw || String(raw));
  }

  async function handleExternalAssistantDelta(payload: unknown) {
    const rawObj = payload && typeof payload === "object" ? payload as Record<string, unknown> : null;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(rawObj?.conversationId || "").trim();
    const parsed = readAssistantEvent(rawObj?.event ?? payload);
    const cacheConversationId = payloadConversationId || currentConversationId;
    if (options.matchesRecentlyCompletedRoundIds(parsed)) {
      return;
    }
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      if (cacheConversationId) {
        if (parsed.streamCache) {
          options.writeConversationStreamCacheSnapshot(cacheConversationId, parsed.streamCache);
          if (parsed.kind === "tool_status") {
            options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
          }
        } else {
          options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
        }
      }
      return;
    }
    // tool_status 是调度层信号，服务头像右侧/运行态提示；它不属于气泡流式结果。
    // 后端将它作为 app-event-only 发送，所以即使 bound channel 已连接也不能在这里去重丢弃。
    if (cacheConversationId) {
      if (parsed.streamCache) {
        options.writeConversationStreamCacheSnapshot(cacheConversationId, parsed.streamCache);
        if (parsed.kind === "tool_status") {
          options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
        }
      } else {
        options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
      }
    }
    if (
      parsed.kind !== "tool_status"
      && assistantEventHasVisibleProgress(parsed)
      && options.channelBinding.hasActiveBoundDeltaChannel(cacheConversationId)
    ) {
      return;
    }
    const round = options.getRound();
    if (
      round.phase === "idle"
      && parsed.kind === "tool_status"
      && !String(parsed.activationId || parsed.requestId || "").trim()
      && options.hasRecentlyCompletedRoundIds()
    ) {
      return;
    }
    if (parsed.kind === "context_usage_update") {
      const currentGen = round.phase === "streaming" || round.phase === "queued" ? round.gen : 0;
      if (currentGen) {
        options.handleStreamingEvent(currentGen, parsed);
      }
      return;
    }
    if (round.phase !== "streaming" && round.phase !== "queued") {
      return;
    }
    const currentGen = round.gen;
    if (!currentGen) {
      return;
    }
    if (parsed.kind === "activity_reasoning_delta") {
      const delta = readDeltaMessage(parsed);
      if (delta && options.reasoningStartedAtMs.value === 0) {
        options.reasoningStartedAtMs.value = Date.now();
      }
    }
    if (parsed.kind === "tool_status") {
      options.handleStreamingEvent(currentGen, parsed);
      return;
    }
    options.handleStreamingEvent(currentGen, parsed);
  }

  return {
    handleExternalAssistantDelta,
    handleExternalHistoryFlushed,
    handleExternalRoundCompleted,
    handleExternalRoundFailed,
    handleExternalRoundStarted,
    handleExternalStreamRebindRequired,
  };
}
