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
  getActiveActivationId: () => string;
  setActiveActivationId: (value: string) => void;
  clearRecentlyCompletedRoundIds: () => void;
  hasRecentlyCompletedRoundIds: () => boolean;
  markRecentlyCompletedRoundIds: (payload: { activationId?: string; requestId?: string } | null | undefined) => void;
  matchesRecentlyCompletedRoundIds: (payload: { activationId?: string; requestId?: string } | null | undefined) => boolean;
  hasStoppedRound: () => boolean;
  matchesStoppedRound: (payload: { assistantMessageId?: string; activationId?: string; requestId?: string }) => boolean;
  clearStoppedRound: () => void;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  getSendChatActiveGen: () => number;
  nextGeneration: () => number;
  channelBinding: {
    bindActiveConversationStream: (conversationId: string, force?: boolean) => Promise<void>;
    hasActiveBoundDeltaChannel: (conversationId?: string | null) => boolean;
    probeBoundChannel: (conversationId?: string | null, timeoutMs?: number) => Promise<boolean>;
    setBoundDisplayGeneration: (gen: number) => void;
  };
  handleHistoryFlushed: (
    gen: number,
    parsed: any,
    source: "sendChat" | "bound",
    options?: { suppressActivationProjection?: boolean },
  ) => Promise<void>;
  beginAssistantActivationFromEvent: (payload: any) => number;
  markRoundStarted: (gen: number) => Promise<void>;
  handleRoundCompleted: (gen: number, result: any) => Promise<void>;
  handleRoundFailed: (
    gen: number,
    error: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) => Promise<void>;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearFrontendDispatchTimer: () => void;
  onReloadMessages: () => Promise<void>;
  onAssistantMessageCompleted?: (input: { conversationId: string; assistantMessage: any }) => Promise<void> | void;
  applyStoppedAssistantMessage?: (assistantMessage: any) => Promise<void> | void;
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
  hasStreamingAssistantMessageInMessages: () => boolean;
  ensureForegroundStreamingRound: () => number;
  handleStreamingEvent: (gen: number, parsed: any) => void;
  syncStreamBlocksToMessage: (messageId: string) => void;
  updateMessageText: (messageId: string) => void;
};

export function externalTerminalTargetsRound(
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

/** 正式历史照常合并；停止只抑制它对旧轮次的等待/流式投影。 */
export function shouldSuppressStoppedHistoryActivation(
  activateAssistant: boolean,
  hasStoppedRound: boolean,
): boolean {
  return activateAssistant && hasStoppedRound;
}

export function useChatFlowExternalEvents(options: UseChatFlowExternalEventsOptions) {
  const STREAM_REBIND_COOLDOWN_MS = 800;
  const rebindCooldownByConversation = new Map<string, number>();
  const rebindInFlightByConversation = new Map<string, Promise<void>>();

  function sameForegroundConversation(payloadConversationId: string): boolean {
    const currentConversationId = options.getCurrentConversationId();
    return !!payloadConversationId
      && !!currentConversationId
      && payloadConversationId === currentConversationId;
  }

  function foregroundAlreadyHandlingCurrentConversation(): boolean {
    const round = options.getRound();
    if (round.phase === "queued" || round.phase === "streaming") {
      return true;
    }
    if (options.hasStreamingAssistantMessageInMessages()) {
      options.ensureForegroundStreamingRound();
      return true;
    }
    return false;
  }

  function terminalTargetsCurrentRound(input: {
    activationId?: string;
    requestId?: string;
    assistantMessageId?: string;
  }): boolean {
    return externalTerminalTargetsRound(
      options.getRound(),
      options.getActiveActivationId(),
      input,
    );
  }

  async function handleExternalStreamRebindRequired(payload: unknown) {
    const raw = payload && typeof payload === "object" ? payload as Record<string, unknown> : null;
    const payloadConversationId = String(raw?.conversationId || "").trim();
    if (!sameForegroundConversation(payloadConversationId)) {
      return;
    }
    if (options.hasStoppedRound()) return;
    if (foregroundAlreadyHandlingCurrentConversation()) {
      return;
    }
    const now = Date.now();
    const lastAt = rebindCooldownByConversation.get(payloadConversationId) || 0;
    if (now - lastAt < STREAM_REBIND_COOLDOWN_MS) {
      return;
    }
    const currentTask = rebindInFlightByConversation.get(payloadConversationId);
    if (currentTask) {
      await currentTask;
      return;
    }
    const rebindTask = (async () => {
      rebindCooldownByConversation.set(payloadConversationId, now);
      if (options.channelBinding.hasActiveBoundDeltaChannel(payloadConversationId)) {
        const probeHealthy = await options.channelBinding.probeBoundChannel(payloadConversationId);
        if (probeHealthy || foregroundAlreadyHandlingCurrentConversation()) {
          return;
        }
      }
      await options.channelBinding.bindActiveConversationStream(payloadConversationId, true);
    })().finally(() => {
      rebindInFlightByConversation.delete(payloadConversationId);
    });
    rebindInFlightByConversation.set(payloadConversationId, rebindTask);
    await rebindTask;
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
    // 正式历史永远不能因停止而丢弃。停止只禁止旧轮次复活；由于
    // historyFlushed 没有轮次身份，先合并消息，并把激活投影延后交给
    // 随后的 roundStarted 按身份精确判断。
    const suppressActivationProjection = shouldSuppressStoppedHistoryActivation(
      !!parsed.activateAssistant,
      options.hasStoppedRound(),
    );
    if (parsed.activateAssistant && !suppressActivationProjection) {
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
      { suppressActivationProjection },
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
    if (options.matchesStoppedRound({
      assistantMessageId: parsed.assistantMessageId,
      activationId: parsed.activationId,
      requestId: parsed.requestId,
    })) {
      return;
    }
    options.clearStoppedRound();
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
    const terminalIdentity = {
      assistantMessageId: parsed.assistantMessage?.id,
      activationId: parsed.activationId,
      requestId: parsed.requestId,
    };
    if (options.matchesStoppedRound(terminalIdentity)) {
      if (parsed.assistantMessage) {
        await options.applyStoppedAssistantMessage?.(parsed.assistantMessage);
      } else if (String(parsed.assistantText || "").trim()) {
        await options.onReloadMessages();
      }
      return;
    }
    options.markRecentlyCompletedRoundIds(parsed);
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.setRound({ phase: "idle" });
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      // 这里是外部终态兜底：当前前台已经不持有该轮次时，仍需走既有对账链路，避免切会话后失去正式历史刷新。
      await options.onReloadMessages();
      return;
    }
    if (!terminalTargetsCurrentRound(terminalIdentity)) return;
    await options.handleRoundCompleted(round.gen, {
      assistantText: String(parsed.assistantText || ""),
      assistantMessage: parsed.assistantMessage,
      activationId: parsed.activationId,
      requestId: parsed.requestId,
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
    if (options.matchesStoppedRound({
      activationId: parsed?.activationId,
      requestId: parsed?.requestId,
    })) return;
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.setRound({ phase: "idle" });
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
    if (!terminalTargetsCurrentRound({
      activationId: parsed?.activationId,
      requestId: parsed?.requestId,
    })) return;
    await options.handleRoundFailed(
      round.gen,
      parsed?.error || raw || String(raw),
      {
        activationId: parsed?.activationId,
        requestId: parsed?.requestId,
      },
    );
  }

  async function handleExternalAssistantDelta(payload: unknown) {
    const rawObj = payload && typeof payload === "object" ? payload as Record<string, unknown> : null;
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(rawObj?.conversationId || "").trim();
    const parsed = readAssistantEvent(rawObj?.event ?? payload);
    const cacheConversationId = payloadConversationId || currentConversationId;
    const round = options.getRound();
    if (options.matchesStoppedRound(parsed)) return;
    if (options.matchesRecentlyCompletedRoundIds(parsed)) {
      return;
    }

    const isAllowedBroadcastDelta =
      parsed.kind === "tool_status"
      || parsed.kind === "context_usage_update";
    if (!isAllowedBroadcastDelta) {
      if (options.debug && assistantEventHasVisibleProgress(parsed)) {
        console.debug("[聊天流程] 已忽略全局高频助手增量事件", {
          currentConversationId,
          payloadConversationId,
          kind: parsed.kind || "delta",
        });
      }
      return;
    }

    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return;
    }
    // tool_status 是调度层信号，服务头像右侧/运行态提示；它不属于气泡流式结果。
    // 后端将它作为低频广播发送，所以即使 bound channel 已连接也不能在这里去重丢弃。
    if (cacheConversationId) {
      options.applyAssistantEventToConversationStreamCache(cacheConversationId, parsed);
    }
    if (
      parsed.kind !== "tool_status"
      && assistantEventHasVisibleProgress(parsed)
      && round.phase !== "idle"
      && options.channelBinding.hasActiveBoundDeltaChannel(cacheConversationId)
    ) {
      return;
    }
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
      options.handleStreamingEvent(currentGen, parsed);
      return;
    }
    if (round.phase !== "streaming" && round.phase !== "queued") {
      if (!assistantEventHasVisibleProgress(parsed)) {
        return;
      }
      const resumedGen = options.ensureForegroundStreamingRound();
      if (!resumedGen) {
        return;
      }
      if (parsed.kind === "activity_reasoning_delta") {
        const delta = readDeltaMessage(parsed);
        if (delta && options.reasoningStartedAtMs.value === 0) {
          options.reasoningStartedAtMs.value = Date.now();
        }
      }
      options.handleStreamingEvent(resumedGen, parsed);
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
