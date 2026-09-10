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
import { probeChatFlow } from "./chat-flow-probe";

type UseChatFlowExternalEventsOptions = {
  debug?: boolean;
  getCurrentConversationId: () => string;
  getActiveActivationId: () => string;
  setActiveActivationId: (value: string) => void;
  clearRecentlyCompletedRoundIds: () => void;
  hasRecentlyCompletedRoundIds: () => boolean;
  markRecentlyCompletedRoundIds: (payload: { activationId?: string; requestId?: string } | null | undefined) => void;
  matchesRecentlyCompletedRoundIds: (payload: { activationId?: string; requestId?: string } | null | undefined) => boolean;
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
  setChatErrorText: (text: string, conversationId?: string | null) => void;
  formatRequestFailed: (error: unknown) => string;
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
  flushStreamTextBuffer?: (gen?: number, messageId?: string) => void;
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

  /**
   * 对账式确保流式订阅：本地无绑定 → 直接 bind；有绑定 → probe 验证
   * channel 与后端登记健康，失败才覆盖式重建。健康时零动作。
   * 同一会话的并发对账复用 in-flight promise，避免重复 bind。
   */
  async function ensureStreamBoundForRound(payloadConversationId: string): Promise<void> {
    const currentTask = rebindInFlightByConversation.get(payloadConversationId);
    if (currentTask) {
      await currentTask;
      return;
    }
    const task = (async () => {
      if (options.channelBinding.hasActiveBoundDeltaChannel(payloadConversationId)) {
        const probeHealthy = await options.channelBinding.probeBoundChannel(payloadConversationId);
        if (probeHealthy) {
          return;
        }
        // probe 失败（channel 失效或后端登记已被清）→ 覆盖式重建，不打断现有显示
      }
      await options.channelBinding.bindActiveConversationStream(payloadConversationId, true);
    })().finally(() => {
      rebindInFlightByConversation.delete(payloadConversationId);
    });
    rebindInFlightByConversation.set(payloadConversationId, task);
    await task;
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
    const now = Date.now();
    const lastAt = rebindCooldownByConversation.get(payloadConversationId) || 0;
    if (now - lastAt < STREAM_REBIND_COOLDOWN_MS) {
      return;
    }
    rebindCooldownByConversation.set(payloadConversationId, now);
    await ensureStreamBoundForRound(payloadConversationId).catch((err) => {
      console.error("[聊天] streamRebindRequired 对账重建失败", {
        conversationId: payloadConversationId,
        message: String((err as { message?: string })?.message ?? err ?? ""),
      });
    });
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
    if (!parsed) {
      probeChatFlow("轮次开始丢弃", { reason: "payload_unparsable", raw: raw.slice(0, 200) });
      return;
    }
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      probeChatFlow("轮次开始丢弃", { reason: "conversation_mismatch", currentConversationId, payloadConversationId });
      return;
    }
    options.clearRecentlyCompletedRoundIds();
    const gen = options.beginAssistantActivationFromEvent(parsed);
    if (!gen) {
      probeChatFlow("轮次开始丢弃", { reason: "gen_zero", payloadConversationId });
      return;
    }
    await options.markRoundStarted(gen);
    // 对账式订阅：任何入口发起的轮次都在广播点确保本窗口绑定健康。
    // 异步执行，不阻塞建气泡；失败由下轮 roundStarted / streamRebindRequired 重试。
    void ensureStreamBoundForRound(payloadConversationId).catch((err) => {
      console.error("[聊天] roundStarted 后流式绑定对账失败", {
        conversationId: payloadConversationId,
        message: String((err as { message?: string })?.message ?? err ?? ""),
      });
    });
  }

  async function handleExternalRoundCompleted(payload: unknown) {
    const raw = stringifyExternalEventPayload(payload, "round_completed");
    const parsed = readRoundCompletedPayload(raw);
    if (!parsed) {
      probeChatFlow("轮次结束丢弃", { reason: "payload_unparsable", raw: raw.slice(0, 200) });
      return;
    }
    const currentConversationId = options.getCurrentConversationId();
    const payloadConversationId = String(parsed.conversationId || "").trim();
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      probeChatFlow("轮次结束丢弃", { reason: "conversation_mismatch", currentConversationId, payloadConversationId });
      options.clearConversationStreamCache(payloadConversationId);
      return;
    }
    const terminalIdentity = {
      assistantMessageId: parsed.assistantMessage?.id,
      activationId: parsed.activationId,
      requestId: parsed.requestId,
    };
    options.markRecentlyCompletedRoundIds(parsed);
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.setRound({ phase: "idle" });
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      options.clearConversationStreamCache(payloadConversationId || currentConversationId);
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      // 外部终态兜底：事件自带后端正式消息时直接应用（停止/他窗口收尾都走这条，
      // 本地已冻结的消息被覆盖一次即可），避免多余的全量重拉；事件没带消息才重拉。
      if (parsed.assistantMessage) {
        await options.onAssistantMessageCompleted?.({
          conversationId: payloadConversationId || currentConversationId,
          assistantMessage: parsed.assistantMessage,
        });
      } else {
        await options.onReloadMessages();
      }
      return;
    }
    if (!terminalTargetsCurrentRound(terminalIdentity)) {
      probeChatFlow("轮次结束丢弃", {
        reason: "terminal_target_mismatch",
        roundPhase: round.phase,
        roundMessageId: round.messageId,
        incomingMessageId: String(terminalIdentity.assistantMessageId || ""),
        incomingActivationId: String(terminalIdentity.activationId || ""),
        incomingRequestId: String(terminalIdentity.requestId || ""),
        currentActivationId: options.getActiveActivationId(),
      });
      return;
    }
    options.flushStreamTextBuffer?.(round.gen, round.messageId);
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
      probeChatFlow("轮次失败丢弃", { reason: "conversation_mismatch", currentConversationId, payloadConversationId });
      const errorDetail = parsed?.error || raw || String(raw);
      options.setChatErrorText(options.formatRequestFailed(errorDetail), payloadConversationId);
      options.clearConversationStreamCache(payloadConversationId);
      return;
    }
    const round = options.getRound();
    if (round.phase !== "streaming" && round.phase !== "queued") {
      options.setRound({ phase: "idle" });
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
    })) {
      probeChatFlow("轮次失败丢弃", {
        reason: "terminal_target_mismatch",
        roundPhase: round.phase,
        roundMessageId: round.messageId,
        incomingActivationId: String(parsed?.activationId || ""),
        incomingRequestId: String(parsed?.requestId || ""),
        currentActivationId: options.getActiveActivationId(),
      });
      return;
    }
    options.flushStreamTextBuffer?.(round.gen, round.messageId);
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
