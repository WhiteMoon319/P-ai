import { describe, expect, it, vi } from "vitest";
import {
  externalTerminalTargetsRound,
} from "./use-chat-flow-external-events";

describe("external chat terminal identity", () => {
  it("rejects a terminal from another activation", () => {
    expect(externalTerminalTargetsRound(
      { phase: "streaming", gen: 2, messageId: "assistant-new" },
      "activation-new",
      { activationId: "activation-old" },
    )).toBe(false);
  });

  it("rejects a formal completion for another assistant message", () => {
    expect(externalTerminalTargetsRound(
      { phase: "streaming", gen: 2, messageId: "assistant-new" },
      "",
      { assistantMessageId: "assistant-old" },
    )).toBe(false);
  });

  it("keeps legacy terminal payloads without identity usable", () => {
    expect(externalTerminalTargetsRound(
      { phase: "queued", gen: 1, messageId: "assistant-1" },
      "activation-1",
      {},
    )).toBe(true);
  });

  it("calls flushStreamTextBuffer before completing the round to prevent truncation", async () => {
    const flushStreamTextBuffer = vi.fn();
    const handleRoundCompleted = vi.fn(async () => {});
    const { useChatFlowExternalEvents } = await import("./use-chat-flow-external-events");
    const external = useChatFlowExternalEvents({
      getCurrentConversationId: () => "conv-1",
      getActiveActivationId: () => "act-1",
      setActiveActivationId: vi.fn(),
      clearRecentlyCompletedRoundIds: vi.fn(),
      hasRecentlyCompletedRoundIds: () => false,
      markRecentlyCompletedRoundIds: vi.fn(),
      matchesRecentlyCompletedRoundIds: () => false,
      getRound: () => ({ phase: "streaming", gen: 1, messageId: "msg-1" }),
      setRound: vi.fn(),
      getSendChatActiveGen: () => 1,
      nextGeneration: () => 2,
      channelBinding: {} as any,
      handleHistoryFlushed: vi.fn(async () => {}),
      beginAssistantActivationFromEvent: vi.fn(() => 1),
      markRoundStarted: vi.fn(async () => {}),
      handleRoundCompleted,
      handleRoundFailed: vi.fn(async () => {}),
      clearConversationStreamCache: vi.fn(),
      clearFrontendDispatchTimer: vi.fn(),
      onReloadMessages: vi.fn(async () => {}),
      setChatErrorText: vi.fn(),
      formatRequestFailed: vi.fn(() => ""),
      chatting: { value: true },
      reasoningStartedAtMs: { value: 0 },
      applyAssistantEventToConversationStreamCache: vi.fn(() => false),
      writeConversationStreamCacheSnapshot: vi.fn(),
      applyConversationStreamCacheToDisplay: vi.fn(() => false),
      hasStreamingAssistantMessageInMessages: () => true,
      ensureForegroundStreamingRound: vi.fn(() => 1),
      handleStreamingEvent: vi.fn(),
      syncStreamBlocksToMessage: vi.fn(),
      updateMessageText: vi.fn(),
      flushStreamTextBuffer,
    });

    await external.handleExternalRoundCompleted({
      conversationId: "conv-1",
      activationId: "act-1",
      assistantText: "完成文本",
    });

    expect(flushStreamTextBuffer).toHaveBeenCalledWith(1, "msg-1");
    expect(handleRoundCompleted).toHaveBeenCalledWith(1, expect.objectContaining({
      assistantText: "完成文本",
    }));
  });
});

describe("roundStarted 订阅对账与重绑守卫", () => {
  async function createExternal(overrides: {
    hasActiveBoundDeltaChannel?: boolean;
    probeHealthy?: boolean;
    roundPhase?: "idle" | "streaming" | "queued";
  } = {}) {
    const bindActiveConversationStream = vi.fn(async () => {});
    const probeBoundChannel = vi.fn(async () => overrides.probeHealthy ?? false);
    const { useChatFlowExternalEvents } = await import("./use-chat-flow-external-events");
    const external = useChatFlowExternalEvents({
      getCurrentConversationId: () => "conv-1",
      getActiveActivationId: () => "act-1",
      setActiveActivationId: vi.fn(),
      clearRecentlyCompletedRoundIds: vi.fn(),
      hasRecentlyCompletedRoundIds: () => false,
      markRecentlyCompletedRoundIds: vi.fn(),
      matchesRecentlyCompletedRoundIds: () => false,
      getRound: () => ({ phase: overrides.roundPhase ?? "streaming", gen: 1, messageId: "msg-1" }),
      setRound: vi.fn(),
      getSendChatActiveGen: () => 1,
      nextGeneration: () => 2,
      channelBinding: {
        bindActiveConversationStream,
        hasActiveBoundDeltaChannel: () => overrides.hasActiveBoundDeltaChannel ?? false,
        probeBoundChannel,
        setBoundDisplayGeneration: vi.fn(),
      },
      handleHistoryFlushed: vi.fn(async () => {}),
      beginAssistantActivationFromEvent: vi.fn(() => 1),
      markRoundStarted: vi.fn(async () => {}),
      handleRoundCompleted: vi.fn(async () => {}),
      handleRoundFailed: vi.fn(async () => {}),
      clearConversationStreamCache: vi.fn(),
      clearFrontendDispatchTimer: vi.fn(),
      onReloadMessages: vi.fn(async () => {}),
      setChatErrorText: vi.fn(),
      formatRequestFailed: vi.fn(() => ""),
      chatting: { value: true },
      reasoningStartedAtMs: { value: 0 },
      applyAssistantEventToConversationStreamCache: vi.fn(() => false),
      writeConversationStreamCacheSnapshot: vi.fn(),
      applyConversationStreamCacheToDisplay: vi.fn(() => false),
      hasStreamingAssistantMessageInMessages: () => true,
      ensureForegroundStreamingRound: vi.fn(() => 1),
      handleStreamingEvent: vi.fn(),
      syncStreamBlocksToMessage: vi.fn(),
      updateMessageText: vi.fn(),
      flushStreamTextBuffer: vi.fn(),
    });
    return { external, bindActiveConversationStream, probeBoundChannel };
  }

  it("本地无绑定时 roundStarted 对账触发 bind", async () => {
    const { external, bindActiveConversationStream } = await createExternal({ hasActiveBoundDeltaChannel: false });
    await external.handleExternalRoundStarted({
      conversationId: "conv-1",
      activationId: "act-1",
      assistantMessageId: "msg-1",
    });
    await vi.waitFor(() => expect(bindActiveConversationStream).toHaveBeenCalledWith("conv-1", true));
  });

  it("有绑定且 probe 健康时 roundStarted 对账不重复 bind", async () => {
    const { external, bindActiveConversationStream } = await createExternal({ hasActiveBoundDeltaChannel: true, probeHealthy: true });
    await external.handleExternalRoundStarted({
      conversationId: "conv-1",
      activationId: "act-1",
      assistantMessageId: "msg-1",
    });
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(bindActiveConversationStream).not.toHaveBeenCalled();
  });

  it("有绑定但 probe 失败时 roundStarted 对账覆盖重建", async () => {
    const { external, bindActiveConversationStream } = await createExternal({ hasActiveBoundDeltaChannel: true, probeHealthy: false });
    await external.handleExternalRoundStarted({
      conversationId: "conv-1",
      activationId: "act-1",
      assistantMessageId: "msg-1",
    });
    await vi.waitFor(() => expect(bindActiveConversationStream).toHaveBeenCalledWith("conv-1", true));
  });

  it("streaming 中 streamRebindRequired probe 失败也必须重建（旧守卫回归）", async () => {
    const { external, bindActiveConversationStream } = await createExternal({
      hasActiveBoundDeltaChannel: true,
      probeHealthy: false,
      roundPhase: "streaming",
    });
    await external.handleExternalStreamRebindRequired({ conversationId: "conv-1" });
    await vi.waitFor(() => expect(bindActiveConversationStream).toHaveBeenCalledWith("conv-1", true));
  });

  it("streaming 中 streamRebindRequired probe 健康时不重建", async () => {
    const { external, bindActiveConversationStream } = await createExternal({
      hasActiveBoundDeltaChannel: true,
      probeHealthy: true,
      roundPhase: "streaming",
    });
    await external.handleExternalStreamRebindRequired({ conversationId: "conv-1" });
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(bindActiveConversationStream).not.toHaveBeenCalled();
  });
});
