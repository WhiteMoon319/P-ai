import { describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import type { ContextUsageUpdatePayload } from "./use-chat-flow-events";
import { useChatFlowStreamingEvents } from "./use-chat-flow-streaming-events";
import type { RoundState } from "./use-chat-flow-types";

function createRuntime(roundInput: RoundState | (() => RoundState), activeActivationId = "activation-new") {
  const getRound = typeof roundInput === "function" ? roundInput : () => roundInput;
  const contextUsagePreview = ref<ContextUsageUpdatePayload | null>({
    conversationId: "conversation-1",
    contextUsagePercent: 10,
    contextUsageRatio: 0.1,
    effectivePromptTokens: 100,
    contextWindowTokens: 1_000,
  });
  const handleRoundCompleted = vi.fn(async () => {});
  const handleRoundFailed = vi.fn(async () => {});
  const setPendingTerminalEvent = vi.fn();
  const clearConversationStreamCache = vi.fn();
  const setActiveActivationId = vi.fn();
  const applyAssistantEventToMessage = vi.fn();
  const enqueueStreamDelta = vi.fn();
  const runtime = useChatFlowStreamingEvents({
    contextUsagePreview,
    reasoningStartedAtMs: ref(0),
    getRound,
    getActiveActivationId: () => activeActivationId,
    promoteQueuedRoundToStreaming: vi.fn((gen: number) => gen),
    setPendingTerminalEvent,
    clearConversationStreamCache,
    getConversationId: () => "conversation-1",
    setActiveActivationId,
    applyConversationStreamCacheSnapshotToDisplay: vi.fn(() => false),
    handleRoundCompleted,
    handleRoundFailed,
    applyAssistantEventToMessage,
    enqueueStreamDelta,
  });

  return {
    ...runtime,
    contextUsagePreview,
    handleRoundCompleted,
    handleRoundFailed,
    setPendingTerminalEvent,
    clearConversationStreamCache,
    setActiveActivationId,
    applyAssistantEventToMessage,
    enqueueStreamDelta,
  };
}

describe("useChatFlowStreamingEvents terminal identity", () => {
  it("ignores a completion from an older activation", () => {
    const runtime = createRuntime({
      phase: "streaming",
      gen: 2,
      messageId: "assistant-new",
    });

    runtime.handleStreamingEvent(2, {
      kind: "round_completed",
      activationId: "activation-old",
      message: JSON.stringify({
        conversationId: "conversation-1",
        activationId: "activation-old",
        assistantText: "旧轮次结果",
      }),
    });

    expect(runtime.handleRoundCompleted).not.toHaveBeenCalled();
  });

  it("ignores a formal completion for another assistant message", () => {
    const runtime = createRuntime({
      phase: "streaming",
      gen: 2,
      messageId: "assistant-new",
    }, "");

    runtime.handleStreamingEvent(2, {
      kind: "round_completed",
      message: JSON.stringify({
        conversationId: "conversation-1",
        assistantText: "旧轮次结果",
        assistantMessage: {
          id: "assistant-old",
          role: "assistant",
          parts: [{ type: "text", text: "旧轮次结果" }],
        },
      }),
    });

    expect(runtime.handleRoundCompleted).not.toHaveBeenCalled();
  });

  it("ignores an older queued failure without clearing the current round state", () => {
    const runtime = createRuntime({
      phase: "queued",
      gen: 2,
      messageId: "assistant-new",
    });
    const preview = runtime.contextUsagePreview.value;

    runtime.handleStreamingEvent(2, {
      kind: "round_failed",
      requestId: "activation-old",
      message: JSON.stringify({
        conversationId: "conversation-1",
        requestId: "activation-old",
        error: "旧轮次失败",
      }),
    });

    expect(runtime.handleRoundFailed).not.toHaveBeenCalled();
    expect(runtime.setPendingTerminalEvent).not.toHaveBeenCalled();
    expect(runtime.clearConversationStreamCache).not.toHaveBeenCalled();
    expect(runtime.setActiveActivationId).not.toHaveBeenCalled();
    expect(runtime.contextUsagePreview.value).toBe(preview);
  });

  it("keeps legacy completion payloads without identity usable", () => {
    const runtime = createRuntime({
      phase: "streaming",
      gen: 2,
      messageId: "assistant-new",
    });

    runtime.handleStreamingEvent(2, {
      kind: "round_completed",
      message: JSON.stringify({
        conversationId: "conversation-1",
        assistantText: "兼容结果",
      }),
    });

    expect(runtime.handleRoundCompleted).toHaveBeenCalledWith(2, {
      assistantText: "兼容结果",
      assistantMessage: undefined,
      activationId: undefined,
      requestId: undefined,
    });
  });

  it("updates context usage preview from stream cache during tool rounds", () => {
    // 钉死：工具执行期间用量随流式缓存（streamCache）下发，前端把最新占用率
    // 写入 contextUsagePreview，圆环实时更新，无需旁路广播 context_usage_update。
    const runtime = createRuntime({
      phase: "streaming",
      gen: 2,
      messageId: "assistant-new",
    });

    runtime.handleStreamingEvent(2, {
      kind: "assistant_tool_result",
      streamCache: {
        persistedAssistantMessageId: "assistant-new",
        contextUsageRatio: 0.42,
        contextUsagePercent: 42,
        effectivePromptTokens: 4200,
        contextWindowTokens: 10_000,
      },
    });

    expect(runtime.contextUsagePreview.value).toEqual({
      conversationId: "conversation-1",
      contextUsagePercent: 42,
      contextUsageRatio: 0.42,
      effectivePromptTokens: 4200,
      contextWindowTokens: 10_000,
      source: "stream_cache",
      eventReason: "provider_tool_round",
    });
  });

  it("does not overwrite preview when stream cache carries no usage", () => {
    // 钉死：无用量字段的 streamCache（旧后端或非工具事件）不得清空或改写 preview。
    const runtime = createRuntime({
      phase: "streaming",
      gen: 2,
      messageId: "assistant-new",
    });
    const before = runtime.contextUsagePreview.value;

    runtime.handleStreamingEvent(2, {
      kind: "assistant_tool_result",
      streamCache: {
        persistedAssistantMessageId: "assistant-new",
      },
    });

    expect(runtime.contextUsagePreview.value).toBe(before);
  });

  it("flushes remaining buffered text when round completes, avoiding truncation", () => {
    const runtime = createRuntime({
      phase: "streaming",
      gen: 3,
      messageId: "assistant-target",
    });

    runtime.handleStreamingEvent(3, { delta: "第一段内容，" } as any);
    runtime.handleStreamingEvent(3, { delta: "最后一段内容" } as any);

    runtime.handleStreamingEvent(3, {
      kind: "round_completed",
      message: JSON.stringify({
        conversationId: "conversation-1",
        assistantText: "完整文本",
      }),
    });

    expect(runtime.enqueueStreamDelta).toHaveBeenCalledWith(3, "第一段内容，最后一段内容");
  });

  it("flushes remaining text directly to message when round phase is no longer streaming", () => {
    let roundState: RoundState = {
      phase: "streaming",
      gen: 1,
      messageId: "assistant-target",
    };
    const runtime = createRuntime(() => roundState);

    runtime.handleStreamingEvent(1, { delta: "残余正文" } as any);
    // 模拟 round 已经先转为 idle（例如被外部调度收尾提前置 idle）
    roundState = { phase: "idle" };
    runtime.flushStreamTextBuffer(1, "assistant-target");

    expect(runtime.applyAssistantEventToMessage).toHaveBeenCalledWith("assistant-target", {
      delta: "残余正文",
    });
  });
});
