import type { ChatMessage } from "../../../types/app";
import { normalizeAssistantStreamBlocks, streamBlocksToToolCalls } from "../../../utils/chat-message-semantics";
import { streamCacheHasVisibleProgress } from "./use-chat-flow-stream-cache";
import { applyStreamingHistoryOverlay } from "./use-chat-flow-stream-overlay";
import { formalizeMessages, normalizeConversationId, positiveRoundedNumber, readMessagePlainText } from "./use-chat-flow-utils";
import type { ResumeForegroundRuntimeRoundInput } from "./use-chat-flow-types";
import type { RoundStartedPayload } from "./use-chat-flow-events";

type ResumeForegroundStreamCacheProjectionInput = {
  conversationId?: string | null;
  reason?: string;
};

export function useChatFlowForegroundRounds(bindings: Record<string, any>) {
  function isStreamingAssistantMessage(message?: ChatMessage | null): boolean {
    const messageId = String(message?.id || "").trim();
    const meta = (message?.providerMeta || {}) as Record<string, unknown>;
    return String(message?.role || "").trim() === "assistant" && meta._streaming === true;
  }

  function messageIdOf(message?: ChatMessage | null): string {
    return String(message?.id || "").trim();
  }

  function findStreamingAssistantMessageById(messageId: string): ChatMessage | null {
    const normalizedMessageId = String(messageId || "").trim();
    if (!normalizedMessageId) return null;
    return [...bindings.allMessages.value]
      .reverse()
      .find((message: ChatMessage) =>
        messageIdOf(message) === normalizedMessageId && isStreamingAssistantMessage(message)
      ) || null;
  }

  function streamCacheMatchesMessageId(cache: unknown, messageId: string): boolean {
    const normalizedMessageId = String(messageId || "").trim();
    if (!normalizedMessageId) return false;
    const cacheMessageId = String((cache as { persistedAssistantMessageId?: unknown } | null | undefined)?.persistedAssistantMessageId || "").trim();
    return !cacheMessageId || cacheMessageId === normalizedMessageId;
  }

  function resolveAssistantMessageId(input?: {
    messageId?: string | null;
    assistantMessageId?: string | null;
    persistedAssistantMessageId?: string | null;
    activationId?: string | null;
    requestId?: string | null;
    gen?: number;
  }): string {
    const explicitMessageId = String(input?.messageId || "").trim();
    if (explicitMessageId) return explicitMessageId;
    const assistantMessageId = String(input?.assistantMessageId || input?.persistedAssistantMessageId || "").trim();
    if (assistantMessageId) return assistantMessageId;
    return "";
  }

  function applyStreamingOverlayForConversation(conversationId?: string | null, expectedMessageId = "") {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return;
    const cache = bindings.readConversationStreamCache(cid);
    if (expectedMessageId && !streamCacheMatchesMessageId(cache, expectedMessageId)) return;
    const overlay = applyStreamingHistoryOverlay(
      bindings.allMessages.value,
      cache,
    );
    if (!overlay.removed) return;
    bindings.allMessages.value = overlay.messages;
    // 流式恢复日志已移除
  }

  function applyQueuedStreamingStateIfNeeded(messageId: string) {
    const queuedStreamingState = bindings.getQueuedStreamingState();
    if (!queuedStreamingState) return;
    bindings.latestAssistantText.value = queuedStreamingState.assistantText;
    bindings.toolStatusText.value = queuedStreamingState.toolStatusText;
    bindings.toolStatusState.value = queuedStreamingState.toolStatusState;
    if (bindings.streamBlocks) {
      bindings.streamBlocks.value = queuedStreamingState.streamBlocks || [];
    }
    if (queuedStreamingState.frontendDispatchStartedAtMs || queuedStreamingState.frontendDispatchElapsedMs) {
      const round = bindings.getRound();
      bindings.startFrontendDispatchTimer(
        round.phase === "queued" || round.phase === "streaming" ? round.gen : 0,
        queuedStreamingState.frontendDispatchStartedAtMs,
        queuedStreamingState.frontendDispatchElapsedMs,
      );
    }
    bindings.setQueuedStreamingState(null);
    bindings.updateMessageText(messageId);
  }

  function beginAssistantActivationFromEvent(payload: RoundStartedPayload): number {
    const payloadConversationId = normalizeConversationId(payload.conversationId);
    const currentConversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return 0;
    }
    const nextActivationId = String(payload.activationId || payload.requestId || "").trim();
    const cid = payloadConversationId || currentConversationId;
    const round = bindings.getRound();
    if (bindings.getActiveActivationId() && nextActivationId && bindings.getActiveActivationId() === nextActivationId && round.phase !== "idle") {
      return round.gen;
    }
    if (cid) bindings.clearConversationStreamCache(cid);
    bindings.setActiveActivationId(nextActivationId);
    const canonicalMessageId = String(payload.assistantMessageId || "").trim();
    if (round.phase === "queued" && !round.messageId && canonicalMessageId) {
      bindings.setRound({ phase: "queued", gen: round.gen, messageId: canonicalMessageId }, "waiting");
    }
    if (cid && positiveRoundedNumber(payload.startedAtMs)) {
      bindings.writeConversationStreamCacheSnapshot(cid, {
        activationId: nextActivationId,
        requestId: String(payload.requestId || nextActivationId || "").trim(),
        departmentId: String(payload.departmentId || "").trim(),
        agentId: String(payload.agentId || "").trim(),
        startedAt: String(payload.startedAt || "").trim(),
        startedAtMs: positiveRoundedNumber(payload.startedAtMs),
        persistedAssistantMessageId: String(payload.assistantMessageId || "").trim(),
      });
    }
    const payloadAgentId = String(payload.agentId || "").trim();
    if (payloadAgentId) bindings.setActiveRoundAgentId?.(payloadAgentId);
    let gen = round.phase === "queued" ? round.gen : bindings.getSendChatActiveGen();
    if (!gen) {
      gen = bindings.nextGeneration();
      bindings.channelBinding.setBoundDisplayGeneration(gen);
      bindings.setPendingTerminalEvent(null);
      bindings.setDeferredRoundCompletion(null);
      bindings.setQueuedStreamingState(null);
      bindings.resetDisplayState();
      bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
      bindings.setRound({
        phase: "queued",
        gen,
        messageId: resolveAssistantMessageId({
          messageId: round.phase === "queued" ? round.messageId : "",
          assistantMessageId: payload.assistantMessageId,
          activationId: nextActivationId,
          requestId: payload.requestId,
          gen,
        }),
      }, "waiting");
    }
    bindings.startFrontendDispatchTimer(
      gen,
      positiveRoundedNumber(payload.startedAtMs) || bindings.sendStartedAtMsByGen.get(gen),
    );
    bindings.chatting.value = true;
    bindings.updateQueuedAssistantMessageStatus(
      resolveAssistantMessageId({
        messageId: bindings.getRound().phase === "queued" ? bindings.getRound().messageId : "",
        assistantMessageId: payload.assistantMessageId,
        activationId: nextActivationId,
        requestId: payload.requestId,
        gen,
      }),
      bindings.t("chat.statusWaitingReply"),
      payload,
    );
    bindings.setFrontendRoundPhase("waiting");
    return gen;
  }

  function cachedDispatchTimerForConversation(): { startedAtMs: number; elapsedMs: number } {
    const conversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    const cache = conversationId ? bindings.readConversationStreamCache(conversationId) : null;
    return {
      startedAtMs: positiveRoundedNumber(cache?.frontendDispatchStartedAtMs || cache?.startedAtMs),
      elapsedMs: positiveRoundedNumber(cache?.frontendDispatchElapsedMs),
    };
  }

  function ensureForegroundWaitingRound(statusText = bindings.t("chat.statusWaitingReply")) {
    const round = bindings.getRound();
    const cachedTimer = cachedDispatchTimerForConversation();
    const conversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    if (round.phase === "queued") {
      bindings.startFrontendDispatchTimer(
        round.gen,
        bindings.frontendDispatch.getStartedAtMs() || cachedTimer.startedAtMs || undefined,
        bindings.frontendDispatch.getElapsedMs() || cachedTimer.elapsedMs,
      );
      bindings.updateQueuedAssistantMessageStatus(round.messageId, statusText);
      bindings.chatting.value = true;
      bindings.setFrontendRoundPhase("waiting");
      return round.gen;
    }
    if (round.phase === "streaming") {
      bindings.startFrontendDispatchTimer(
        round.gen,
        bindings.frontendDispatch.getStartedAtMs() || cachedTimer.startedAtMs || undefined,
        bindings.frontendDispatch.getElapsedMs() || cachedTimer.elapsedMs,
      );
      if (!bindings.hasStreamingAssistantMessageInMessages()) {
        const messageId = bindings.insertStreamingAssistantMessage(round.messageId, round.gen, statusText);
        bindings.updateMessageText(messageId);
        bindings.setRound({ phase: "streaming", gen: round.gen, messageId });
      }
      bindings.chatting.value = true;
      return round.gen;
    }
    const gen = bindings.nextGeneration();
    bindings.channelBinding.setBoundDisplayGeneration(gen);
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
    const messageId = resolveAssistantMessageId({
      persistedAssistantMessageId: conversationId ? bindings.readConversationStreamCache(conversationId)?.persistedAssistantMessageId : "",
      activationId: cachedTimer.startedAtMs > 0 ? bindings.getActiveActivationId?.() : "",
      requestId: conversationId ? bindings.readConversationStreamCache(conversationId)?.requestId : "",
      gen,
    });
    bindings.setRound({ phase: "queued", gen, messageId }, "waiting");
    bindings.startFrontendDispatchTimer(gen, cachedTimer.startedAtMs || undefined, cachedTimer.elapsedMs);
    bindings.chatting.value = true;
    bindings.updateQueuedAssistantMessageStatus(messageId, statusText);
    return gen;
  }

  function ensureForegroundStreamingRound() {
    const conversationId = bindings.getConversationId ? bindings.getConversationId() : "";
    const round = bindings.getRound();
    const cache = conversationId ? bindings.readConversationStreamCache(conversationId) : null;
    if (round.phase === "streaming") {
      const targetMessageId = round.messageId;
      const existingMessage = findStreamingAssistantMessageById(targetMessageId);
      if (!existingMessage) {
        if (bindings.streamBlocks) bindings.streamBlocks.value = [];
        const restoredFromCache = !!(targetMessageId
          && streamCacheMatchesMessageId(cache, targetMessageId)
          && bindings.applyConversationStreamCacheToDisplay(conversationId));
        const messageId = bindings.insertStreamingAssistantMessage(targetMessageId, round.gen);
        if (!restoredFromCache) bindings.latestAssistantText.value = "";
        bindings.updateMessageText(messageId);
        bindings.setRound({ phase: "streaming", gen: round.gen, messageId });
      }
      return round.gen;
    }
    const targetMessageId = round.phase === "queued"
      ? round.messageId
      : String(cache?.persistedAssistantMessageId || "").trim();
    if (!targetMessageId) return 0;
    const gen = bindings.nextGeneration();
    if (bindings.streamBlocks) bindings.streamBlocks.value = [];
    const existingMessage = findStreamingAssistantMessageById(targetMessageId);
    const existingMessageId = messageIdOf(existingMessage);
    const existingMessageMeta = ((existingMessage?.providerMeta || {}) as Record<string, unknown>);
    const restoredFromCache = !!(!existingMessageId
      && targetMessageId
      && streamCacheMatchesMessageId(cache, targetMessageId)
      && bindings.applyConversationStreamCacheToDisplay(conversationId));
    applyStreamingOverlayForConversation(conversationId, targetMessageId);
    const existingMessageStartedAtMs = existingMessageId ? positiveRoundedNumber(existingMessageMeta._frontendDispatchStartedAtMs) : 0;
    const existingMessageElapsedMs = existingMessageId ? positiveRoundedNumber(existingMessageMeta._frontendDispatchElapsedMs) : 0;
    if (!restoredFromCache) {
      bindings.latestAssistantText.value = readMessagePlainText(existingMessage || undefined);
    }
    bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
    const messageId = existingMessageId || bindings.insertStreamingAssistantMessage(targetMessageId, gen);
    if (existingMessageId) {
      bindings.loadStreamBlocksFromMessage(messageId);
    }
    if (existingMessageId || restoredFromCache) {
      bindings.updateMessageText(messageId);
    }
    bindings.setRound({ phase: "streaming", gen, messageId });
    bindings.startFrontendDispatchTimer(
      gen,
      existingMessageStartedAtMs || bindings.frontendDispatch.getStartedAtMs() || undefined,
      existingMessageElapsedMs || bindings.frontendDispatch.getElapsedMs(),
    );
    bindings.chatting.value = true;
    applyQueuedStreamingStateIfNeeded(messageId);
    return gen;
  }

  function resumeForegroundRuntimeRound(input?: ResumeForegroundRuntimeRoundInput) {
    const conversationId = normalizeConversationId(input?.conversationId || (bindings.getConversationId ? bindings.getConversationId() : ""));
    if (!conversationId) return 0;
    const snapshotBlocks = normalizeAssistantStreamBlocks(input?.streamCache?.streamBlocks || []);
    if (input?.streamCache) {
      bindings.writeConversationStreamCacheSnapshot(conversationId, input.streamCache);
    }
    const cache = bindings.readConversationStreamCache(conversationId);
    const round = bindings.getRound();
    const expectedMessageId = round.phase === "queued" || round.phase === "streaming"
      ? round.messageId
      : String(cache?.persistedAssistantMessageId || "").trim();
    applyStreamingOverlayForConversation(conversationId, expectedMessageId);
    const hasVisibleProgress =
      !!input?.streamCache?.hasVisibleProgress
      || streamCacheHasVisibleProgress(input?.streamCache)
      || streamCacheHasVisibleProgress(cache);
    // 应用后端运行态快照日志已移除
    if (!hasVisibleProgress) {
      return ensureForegroundWaitingRound(input?.statusText || bindings.t("chat.statusWaitingReply"));
    }
    const gen = ensureForegroundStreamingRound();
    const nextRound = bindings.getRound();
    if (nextRound.phase === "streaming") {
      const blocks = snapshotBlocks.length > 0 ? snapshotBlocks : normalizeAssistantStreamBlocks(cache?.streamBlocks || []);
      if (bindings.streamBlocks) bindings.streamBlocks.value = blocks;
      bindings.syncStreamBlocksToMessage(nextRound.messageId, blocks);
      bindings.updateMessageText(nextRound.messageId, undefined, undefined, "", blocks);
    }
    return gen;
  }

  function resumeForegroundStreamCacheProjection(input?: ResumeForegroundStreamCacheProjectionInput) {
    const currentConversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    const conversationId = normalizeConversationId(input?.conversationId || currentConversationId);
    if (!conversationId || conversationId !== currentConversationId) return 0;
    const cache = bindings.readConversationStreamCache(conversationId);
    if (!streamCacheHasVisibleProgress(cache)) return 0;
    // 从前端缓存恢复当前会话投影日志已移除
    return ensureForegroundStreamingRound();
  }

  function promoteQueuedRoundToStreaming(gen: number) {
    const round = bindings.getRound();
    if (round.phase === "streaming" && round.gen === gen) {
      return gen;
    }
    if (round.phase !== "queued" || round.gen !== gen) {
      return 0;
    }
    const conversationId = bindings.getConversationId ? bindings.getConversationId() : "";
    if (bindings.streamBlocks) bindings.streamBlocks.value = [];
    const cache = conversationId ? bindings.readConversationStreamCache(conversationId) : null;
    const targetMessageId = round.messageId;
    const existingMessage = findStreamingAssistantMessageById(targetMessageId);
    const existingMessageId = messageIdOf(existingMessage);
    const existingMessageMeta = ((existingMessage?.providerMeta || {}) as Record<string, unknown>);
    const restoredFromCache = !!(!existingMessageId
      && targetMessageId
      && streamCacheMatchesMessageId(cache, targetMessageId)
      && bindings.applyConversationStreamCacheToDisplay(conversationId));
    applyStreamingOverlayForConversation(conversationId, targetMessageId);
    const existingMessageStartedAtMs = existingMessageId ? positiveRoundedNumber(existingMessageMeta._frontendDispatchStartedAtMs) : 0;
    const existingMessageElapsedMs = existingMessageId ? positiveRoundedNumber(existingMessageMeta._frontendDispatchElapsedMs) : 0;
    if (!restoredFromCache) {
      bindings.latestAssistantText.value = readMessagePlainText(existingMessage || undefined);
    }
    bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
    const messageId = existingMessageId || bindings.insertStreamingAssistantMessage(targetMessageId, gen);
    if (existingMessageId) {
      bindings.loadStreamBlocksFromMessage(messageId);
    }
    if (existingMessageId || restoredFromCache) {
      bindings.updateMessageText(messageId);
    }
    bindings.setRound({ phase: "streaming", gen, messageId });
    bindings.startFrontendDispatchTimer(
      gen,
      existingMessageStartedAtMs || bindings.frontendDispatch.getStartedAtMs() || undefined,
      existingMessageElapsedMs || bindings.frontendDispatch.getElapsedMs(),
    );
    bindings.chatting.value = true;
    applyQueuedStreamingStateIfNeeded(messageId);
    bindings.applyPendingTerminalEvent(gen);
    return gen;
  }

  return {
    beginAssistantActivationFromEvent,
    ensureForegroundWaitingRound,
    ensureForegroundStreamingRound,
    resumeForegroundRuntimeRound,
    resumeForegroundStreamCacheProjection,
    promoteQueuedRoundToStreaming,
  };
}
