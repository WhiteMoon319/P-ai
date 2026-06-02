import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { computed, ref, shallowRef } from "vue";
import type { AssistantStreamBlock, ChatMessage } from "../src/types/app";
import { useChatFlow, type AssistantDeltaEvent } from "../src/features/chat/composables/use-chat-flow";
import { useChatRuntime } from "../src/features/chat/composables/use-chat-runtime";
import { useChatMessageBlocks } from "../src/features/chat/composables/use-chat-turns";
import { projectMessageForDisplay } from "../src/utils/chat-message-semantics";

const hoisted = vi.hoisted(() => {
  class MockChannel<T> {
    onmessage?: (event: T) => void;

    emit(event: T) {
      this.onmessage?.(event);
    }
  }

  return {
    MockChannel,
    invokeTauriMock: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  Channel: hoisted.MockChannel,
}));

vi.mock("../src/services/tauri-api", () => ({
  invokeTauri: hoisted.invokeTauriMock,
}));

function textMessage(id: string, role: "user" | "assistant", text: string): ChatMessage {
  return {
    id,
    role,
    parts: [{ type: "text", text }],
  };
}

async function flushAsyncSteps(times = 4) {
  // history_flushed 处理链路里包含一个 fire-and-forget async IIFE，
  // 内部还会 await onReloadMessages()，因此这里主动多冲几轮微任务，
  // 让测试在断言前稳定等到“刷新历史 -> 切换 chatting”这条链走完。
  for (let idx = 0; idx < times; idx += 1) {
    await Promise.resolve();
  }
}

describe("useChatFlow stream isolation", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    hoisted.invokeTauriMock.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("closes queued round immediately when compaction boundary completes it", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };

    let capturedChannel: ChannelLike | null = null;
    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise(() => {
          capturedChannel = onDelta as unknown as ChannelLike;
        }),
      onReloadMessages,
    });

    void flow.sendChat();
    await Promise.resolve();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    expect(flow.frontendRoundPhase.value).toBe("waiting");

    capturedChannel!.emit({
      kind: "round_completed",
      reason: "context_compaction_boundary",
      message: "{\"conversationId\":\"conversation-1\",\"assistantText\":\"\",\"archivedBeforeSend\":false}",
    });
    await flushAsyncSteps();

    expect(flow.frontendRoundPhase.value).toBe("idle");
    expect(chatting.value).toBe(false);
    expect(allMessages.value.some((message) => String(message.id || "").startsWith("__draft_assistant__:"))).toBe(false);
    expect(onReloadMessages).toHaveBeenCalledTimes(1);
  });

  it("does not hydrate streaming bubble from history before first delta", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);

    const oldHistory: ChatMessage[] = [
      textMessage("u-old", "user", "old question"),
      textMessage("a-old", "assistant", "A_old"),
    ];

    hoisted.invokeTauriMock.mockImplementation(async (command: string) => {
      if (command === "get_foreground_conversation_light_snapshot") {
        return { messages: oldHistory };
      }
      throw new Error(`unexpected invoke command: ${command}`);
    });

    const runtime = useChatRuntime({
      t: (key) => key,
      setStatus: () => {},
      setStatusError: () => {},
      setChatError: () => {},
      activeChatApiConfigId: ref("api-1"),
      assistantDepartmentAgentId: ref("agent-1"),
      chatting,
      trimming,
      compactingConversation: ref(false),
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      perfNow: () => Date.now(),
      perfLog: () => {},
      perfDebug: false,
    });

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };

    let capturedChannel: ChannelLike | null = null;
    let resolveRequest:
      | ((value: {
        assistantText: string;
        latestUserText: string;
        archivedBeforeSend: boolean;
      }) => void)
      | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise((resolve) => {
          capturedChannel = onDelta as unknown as ChannelLike;
          resolveRequest = resolve;
        }),
      onReloadMessages: () => runtime.refreshConversationHistory(),
    });

    const sendPromise = flow.sendChat();
    await Promise.resolve();

    expect(chatting.value).toBe(false);
    expect(latestAssistantText.value).toBe("");

    await runtime.refreshConversationHistory();
    expect(allMessages.value).toEqual(oldHistory);
    expect(latestAssistantText.value).toBe("");

    expect(capturedChannel).not.toBeNull();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    expect(chatting.value).toBe(true);
    expect(flow.frontendRoundPhase.value).toBe("waiting");
    expect(visibleTurnCount.value).toBe(1);

    capturedChannel!.emit({ delta: "N" });
    await vi.advanceTimersByTimeAsync(34);
    expect(chatting.value).toBe(true);
    expect(latestAssistantText.value).toBe("N");

    expect(resolveRequest).not.toBeNull();
    resolveRequest!({
      assistantText: "A_new",
      latestUserText: "new question",
      archivedBeforeSend: false,
    });

    await sendPromise;

    expect(latestAssistantText.value).toBe("N");
    expect(chatErrorText.value).toBe("");
    expect(chatting.value).toBe(true);
  });

  it("shows retry status in the pre-streaming assistant draft", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };

    let capturedChannel: ChannelLike | null = null;
    let resolveRequest:
      | ((value: {
        assistantText: string;
        latestUserText: string;
        archivedBeforeSend: boolean;
      }) => void)
      | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise((resolve) => {
          capturedChannel = onDelta as unknown as ChannelLike;
          resolveRequest = resolve;
        }),
      onReloadMessages: async () => {},
    });

    const sendPromise = flow.sendChat();
    await Promise.resolve();

    expect(capturedChannel).not.toBeNull();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();

    capturedChannel!.emit({
      kind: "tool_status",
      toolStatus: "running",
      message: "模型请求失败 code 500，正在重试 (1/5)，等待 1 秒...",
    });

    const assistantDraft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect(assistantDraft?.providerMeta?._preStreamingStatusText).toBe("模型请求失败 code 500，正在重试 (1/5)，等待 1 秒...");
    expect(toolStatusText.value).toBe("模型请求失败 code 500，正在重试 (1/5)，等待 1 秒...");

    capturedChannel!.emit({ delta: "N" });
    await vi.advanceTimersByTimeAsync(34);

    const streamingDraft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect(streamingDraft?.providerMeta?._preStreamingStatusText).toBe("");

    expect(resolveRequest).not.toBeNull();
    resolveRequest!({
      assistantText: "A_new",
      latestUserText: "new question",
      archivedBeforeSend: false,
    });

    await sendPromise;
  });

  it("clears retry waiting draft after stop succeeds", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});
    const invokeStopChatMessage = vi.fn(async () => ({ aborted: true, persisted: false }));

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };

    let capturedChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise(() => {
          capturedChannel = onDelta as unknown as ChannelLike;
        }),
      invokeStopChatMessage,
      onReloadMessages,
    });

    void flow.sendChat();
    await Promise.resolve();

    expect(capturedChannel).not.toBeNull();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    capturedChannel!.emit({
      kind: "tool_status",
      toolStatus: "running",
      message: "模型请求失败 code 500，正在重试 (1/5)，等待 1 秒...",
    });

    expect(chatting.value).toBe(true);
    expect(allMessages.value.some((message) => String(message.id || "").startsWith("__draft_assistant__:"))).toBe(true);

    await flow.stopChat();

    expect(invokeStopChatMessage).toHaveBeenCalledTimes(1);
    expect(chatting.value).toBe(false);
    expect(flow.frontendRoundPhase.value).toBe("idle");
    expect(toolStatusText.value).toBe("");
    expect(toolStatusState.value).toBe("");
    expect(allMessages.value.some((message) => String(message.id || "").startsWith("__draft_assistant__:"))).toBe(false);
    expect(onReloadMessages).toHaveBeenCalledTimes(1);
  });

  it("stops stream by preserving partial text and syncing stop payload", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});
    const invokeStopChatMessage = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let capturedChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise(() => {
          capturedChannel = onDelta as unknown as ChannelLike;
        }),
      invokeStopChatMessage,
      onReloadMessages,
    });

    void flow.sendChat();
    await Promise.resolve();
    expect(chatting.value).toBe(false);

    expect(capturedChannel).not.toBeNull();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    expect(chatting.value).toBe(true);
    expect(flow.frontendRoundPhase.value).toBe("waiting");
    expect(visibleTurnCount.value).toBe(1);
    capturedChannel!.emit({
      delta: "ABC",
      streamCache: {
        assistantText: "ABC",
        streamBlocks: [{ text: "ABC" }],
      },
    });
    capturedChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "R1",
      streamCache: {
        assistantText: "ABC",
        streamBlocks: [{ reasoning: "R1", text: "ABC" }],
      },
    });
    expect(chatting.value).toBe(true);

    await flow.stopChat();

    expect(chatting.value).toBe(false);
    expect(invokeStopChatMessage).toHaveBeenCalledTimes(1);
    expect(invokeStopChatMessage).toHaveBeenCalledWith({
      session: { apiConfigId: "api-1", agentId: "agent-1", conversationId: "conversation-1" },
      partialAssistantText: "ABC",
      partialStreamBlocks: [{ reasoning: "R1", text: "ABC", tools: [] }],
    });
    expect(onReloadMessages).toHaveBeenCalledTimes(1);
  });

  it("keeps streamed reasoning on final assistant messages without tools", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let capturedChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise(() => {
          capturedChannel = onDelta as unknown as ChannelLike;
        }),
      onReloadMessages,
    });

    void flow.sendChat();
    await Promise.resolve();
    expect(capturedChannel).not.toBeNull();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    capturedChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "先判断用户提到的工具指代。",
      streamCache: {
        assistantText: "",
        streamBlocks: [{ reasoning: "先判断用户提到的工具指代。" }],
      },
    });
    capturedChannel!.emit({
      delta: "不太确定，展开说说？",
      streamCache: {
        assistantText: "不太确定，展开说说？",
        streamBlocks: [{ reasoning: "先判断用户提到的工具指代。", text: "不太确定，展开说说？" }],
      },
    });
    capturedChannel!.emit({
      kind: "round_completed",
      message: JSON.stringify({
        conversationId: "conversation-1",
        assistantText: "不太确定，展开说说？",
        archivedBeforeSend: false,
        assistantMessage: {
          ...textMessage("a-final", "assistant", "不太确定，展开说说？"),
          toolCall: [{
            role: "assistant",
            content: "不太确定，展开说说？",
            reasoning_content: "先判断用户提到的工具指代。",
          }],
        },
      }),
    });
    await flushAsyncSteps();

    const finalMessage = allMessages.value.find((message) => message.id === "a-final");
    expect(finalMessage?.activityItems).toBeUndefined();
    expect(finalMessage?.toolCall?.[0]).toMatchObject({
      role: "assistant",
      reasoning_content: "先判断用户提到的工具指代。",
    });
    expect(streamBlocks.value).toEqual([]);
    expect(chatting.value).toBe(false);
  });

  it("does not synthesize reasoning or tools from empty stream block snapshots", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let capturedChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise(() => {
          capturedChannel = onDelta as unknown as ChannelLike;
        }),
      onReloadMessages,
    });

    void flow.sendChat();
    await Promise.resolve();
    expect(capturedChannel).not.toBeNull();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();

    capturedChannel!.emit({
      kind: "assistant_tool_event",
      message: JSON.stringify({
        role: "assistant",
        content: null,
        reasoning_content: "打算用 operate 工具等待 3 秒。",
        tool_calls: [{
          id: "tool-1",
          call_id: "tool-1",
          type: "function",
          function: {
            name: "operate",
            arguments: "{\"action\":\"wait\",\"seconds\":3}",
          },
        }],
      }),
      streamCache: {
        assistantText: "",
        streamBlocks: [],
      },
    });
    capturedChannel!.emit({
      kind: "tool_status",
      toolName: "operate",
      toolCallId: "tool-1",
      toolStatus: "running",
      toolArgs: "{\"action\":\"wait\",\"seconds\":3}",
      message: "正在执行 operate",
      streamCache: {
        assistantText: "",
        streamBlocks: [],
      },
    });

    expect(streamBlocks.value).toEqual([]);
    expect(toolStatusText.value).toBe("正在执行 operate");
    expect(toolStatusState.value).toBe("running");

    const draft = allMessages.value.find((message) => message.role === "assistant" && message.id.startsWith("__draft_assistant__:"));
    expect((draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks).toEqual([]);
    const projection = projectMessageForDisplay(draft as ChatMessage);
    expect(projection.activityItems).toEqual([]);

    const { visibleMessageBlocks } = useChatMessageBlocks({
      allMessages,
      activeChatApiConfig: computed(() => null),
      perfDebug: false,
      perfNow: () => 0,
    });
    const draftBlock = visibleMessageBlocks.value.find((block) => String(block.id || "").startsWith("__draft_assistant__:"));
    expect(draftBlock?.activityItems).toEqual([]);
    expect(draftBlock?.activityStatus).toBe("requesting");
  });

  it("shows external reasoning deltas on the active streaming draft", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let capturedChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise(() => {
          capturedChannel = onDelta as unknown as ChannelLike;
        }),
      onReloadMessages,
    });

    void flow.sendChat();
    await Promise.resolve();
    capturedChannel!.emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    capturedChannel!.emit({
      delta: "A",
      streamCache: {
        assistantText: "A",
        streamBlocks: [{ text: "A" }],
      },
    });
    await flushAsyncSteps();

    await flow.handleExternalAssistantDelta({
      conversationId: "conversation-1",
      event: {
        kind: "activity_reasoning_delta",
        delta: "外部流式思考",
        streamCache: {
          assistantText: "A",
          streamBlocks: [{ reasoning: "外部流式思考", text: "A" }],
        },
      },
    });
    await flushAsyncSteps();

    const draft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect(streamBlocks.value).toEqual([{ reasoning: "外部流式思考", text: "A", tools: [] }]);
    expect((draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks).toEqual(streamBlocks.value);
  });

  it("keeps current streaming round visible until history_flushed switches to next round", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("first question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };

    const capturedChannels: ChannelLike[] = [];
    const resolveRequests: Array<(value: {
      assistantText: string;
      latestUserText: string;
      archivedBeforeSend: boolean;
    }) => void> = [];

    const flow = useChatFlow({
      chatting,
      trimming,
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: ({ onDelta }) =>
        new Promise((resolve) => {
          capturedChannels.push(onDelta as unknown as ChannelLike);
          resolveRequests.push(resolve);
        }),
      onReloadMessages,
    });

    const firstSend = flow.sendChat();
    await Promise.resolve();
    expect(chatting.value).toBe(false);
    expect(capturedChannels).toHaveLength(1);

    capturedChannels[0].emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":1,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    expect(chatting.value).toBe(true);
    expect(flow.frontendRoundPhase.value).toBe("waiting");
    expect(visibleTurnCount.value).toBe(1);

    capturedChannels[0].emit({ delta: "FIRST" });
    await vi.advanceTimersByTimeAsync(250);
    expect(chatting.value).toBe(true);
    expect(latestAssistantText.value).toBe("FIRST");

    chatInput.value = "second question";
    const secondSend = flow.sendChat();
    await Promise.resolve();
    expect(capturedChannels).toHaveLength(2);

    // 第二次发送只是在排队，不能抢占当前正在显示的第一轮流式。
    expect(chatting.value).toBe(true);
    expect(latestAssistantText.value).toBe("FIRST");

    capturedChannels[1].emit({ delta: "SECOND-BEFORE-FLUSH" });
    await vi.advanceTimersByTimeAsync(250);
    expect(latestAssistantText.value).toBe("FIRST");

    capturedChannels[1].emit({ kind: "history_flushed", message: "{\"conversationId\":\"conversation-1\",\"messageCount\":2,\"activateAssistant\":true}" });
    await flushAsyncSteps();
    expect(onReloadMessages).toHaveBeenCalledTimes(0);
    expect(latestAssistantText.value).toBe("FIRST");
    expect(chatting.value).toBe(true);
    expect(flow.frontendRoundPhase.value).toBe("waiting");
    expect(visibleTurnCount.value).toBe(1);

    capturedChannels[1].emit({ delta: "SECOND-AFTER-FLUSH" });
    await vi.advanceTimersByTimeAsync(1200);
    expect(latestAssistantText.value).toBe("SECOND-AFTER-FLUSH");

    resolveRequests[0]({
      assistantText: "FIRST-DONE",
      latestUserText: "first question",
      archivedBeforeSend: false,
    });
    await firstSend;

    // 第二轮收到 history_flushed 后成为前台轮次，第一轮收尾不再覆盖当前显示。
    expect(latestAssistantText.value).toBe("SECOND-AFTER-FLUSH");
    expect(chatting.value).toBe(true);

    resolveRequests[1]({
      assistantText: "SECOND-DONE",
      latestUserText: "second question",
      archivedBeforeSend: false,
    });
    await secondSend;

    expect(latestAssistantText.value).toBe("SECOND-AFTER-FLUSH");
    expect(chatting.value).toBe(true);
  });

  it("does not enter streaming view for non-activated batch without history_flushed", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("queued-only");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    let resolveRequest:
      | ((value: {
        assistantText: string;
        latestUserText: string;
        archivedBeforeSend: boolean;
      }) => void)
      | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: () =>
        new Promise((resolve) => {
          resolveRequest = resolve;
        }),
      onReloadMessages,
    });

    const sendPromise = flow.sendChat();
    await Promise.resolve();

    // 仅入队、未收到 history_flushed 时，不应出现新的前台流式轮次。
    expect(chatting.value).toBe(false);
    expect(latestAssistantText.value).toBe("");

    resolveRequest!({
      assistantText: "",
      latestUserText: "queued-only",
      archivedBeforeSend: false,
    });
    await sendPromise;

    expect(latestAssistantText.value).toBe("");
    expect(onReloadMessages).toHaveBeenCalledTimes(0);
    expect(chatting.value).toBe(false);
  });

  it("projects bound channel stream blocks while the send round is still queued", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let boundChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: vi.fn(async () => ({
        accepted: true,
        duplicate: false,
        eventId: "event-1",
        conversationId: "conversation-1",
        traceId: "trace-1",
        ingress: "accepted",
      })),
      invokeBindActiveChatViewStream: vi.fn(async ({ onDelta }) => {
        boundChannel = onDelta as unknown as ChannelLike;
      }),
      onReloadMessages,
    });

    await flow.bindActiveConversationStream("conversation-1");
    await flow.sendChat();
    expect(flow.frontendRoundPhase.value).toBe("queued");
    expect(boundChannel).not.toBeNull();

    boundChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "正在分析流式块。",
      streamCache: {
        assistantText: "你好",
        streamBlocks: [{ reasoning: "正在分析流式块。", text: "你好" }],
      },
    });
    await flushAsyncSteps();

    expect(flow.frontendRoundPhase.value).toBe("streaming");
    expect(streamBlocks.value).toEqual([{
      reasoning: "正在分析流式块。",
      text: "你好",
      tools: [],
    }]);
    const draft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect((draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks).toEqual(streamBlocks.value);
    expect(projectMessageForDisplay(draft as ChatMessage).activityItems.map((item) => item.text)).toEqual([
      "正在分析流式块。",
    ]);
  });

  it("projects active stream snapshots even when runtime activation ids differ", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let boundChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: vi.fn(async () => ({
        accepted: true,
        duplicate: false,
        eventId: "event-1",
        conversationId: "conversation-1",
        traceId: "trace-1",
        ingress: "accepted",
      })),
      invokeBindActiveChatViewStream: vi.fn(async ({ onDelta }) => {
        boundChannel = onDelta as unknown as ChannelLike;
      }),
      onReloadMessages,
    });

    await flow.bindActiveConversationStream("conversation-1");
    await flow.sendChat();
    boundChannel!.emit({
      kind: "activity_reasoning_delta",
      activationId: "backend-activation",
      requestId: "backend-activation",
      delta: "正在分析不同 activation 的当前通道事件。",
      streamCache: {
        activationId: "backend-activation",
        requestId: "backend-activation",
        assistantText: "",
        streamBlocks: [{ reasoning: "正在分析不同 activation 的当前通道事件。" }],
      },
    });
    await flushAsyncSteps();

    expect(streamBlocks.value).toEqual([{
      reasoning: "正在分析不同 activation 的当前通道事件。",
      text: "",
      tools: [],
    }]);
    const draft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect((draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks).toEqual(streamBlocks.value);
    expect(projectMessageForDisplay(draft as ChatMessage).activityItems.map((item) => item.text)).toEqual([
      "正在分析不同 activation 的当前通道事件。",
    ]);
  });

  it("projects stream snapshots into the draft without relying on the streamBlocks ref", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let boundChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: vi.fn(async () => ({
        accepted: true,
        duplicate: false,
        eventId: "event-1",
        conversationId: "conversation-1",
        traceId: "trace-1",
        ingress: "accepted",
      })),
      invokeBindActiveChatViewStream: vi.fn(async ({ onDelta }) => {
        boundChannel = onDelta as unknown as ChannelLike;
      }),
      onReloadMessages,
    });

    await flow.bindActiveConversationStream("conversation-1");
    await flow.sendChat();
    boundChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "直接从快照写入草稿。",
      streamCache: {
        assistantText: "",
        streamBlocks: [{ reasoning: "直接从快照写入草稿。" }],
      },
    });
    await flushAsyncSteps();

    const draft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect((draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks).toEqual([{
      reasoning: "直接从快照写入草稿。",
      text: "",
      tools: [],
    }]);
    expect(projectMessageForDisplay(draft as ChatMessage).activityItems.map((item) => item.text)).toEqual([
      "直接从快照写入草稿。",
    ]);
  });

  it("keeps prior reasoning when a tool snapshot and later reasoning arrive", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let boundChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: vi.fn(async () => ({
        accepted: true,
        duplicate: false,
        eventId: "event-1",
        conversationId: "conversation-1",
        traceId: "trace-1",
        ingress: "accepted",
      })),
      invokeBindActiveChatViewStream: vi.fn(async ({ onDelta }) => {
        boundChannel = onDelta as unknown as ChannelLike;
      }),
      onReloadMessages,
    });

    await flow.bindActiveConversationStream("conversation-1");
    await flow.sendChat();
    boundChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "思维链1",
      streamCache: {
        assistantText: "",
        streamBlocks: [{ reasoning: "思维链1" }],
      },
    });
    await flushAsyncSteps();
    boundChannel!.emit({
      kind: "assistant_tool_event",
      message: JSON.stringify({
        role: "assistant",
        content: null,
        tool_calls: [{
          id: "tool-1",
          type: "function",
          function: {
            name: "operate",
            arguments: "{\"action\":\"wait\"}",
          },
        }],
      }),
      streamCache: {
        assistantText: "",
        streamBlocks: [{
          reasoning: "思维链1",
          tools: [{
            toolCallId: "tool-1",
            name: "operate",
            argsText: "{\"action\":\"wait\"}",
            status: "doing",
          }],
        }],
      },
    });
    await flushAsyncSteps();
    boundChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "思维链2",
      streamCache: {
        assistantText: "",
        streamBlocks: [{
          reasoning: "思维链1",
          tools: [{
            toolCallId: "tool-1",
            name: "operate",
            argsText: "{\"action\":\"wait\"}",
            status: "doing",
          }],
        }, { reasoning: "思维链2" }],
      },
    });
    await flushAsyncSteps();

    const draft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    const blocks = (draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks;
    expect(blocks).toEqual([{
      reasoning: "思维链1",
      text: "",
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\"}",
        status: "doing",
      }],
    }, {
      reasoning: "思维链2",
      text: "",
      tools: [],
    }]);
    const projection = projectMessageForDisplay(draft as ChatMessage);
    expect(projection.activityItems.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "思维链1",
      "operate",
      "思维链2",
    ]);
  });

  it("does not let duplicated app tool events clear a bound-channel draft", async () => {
    const chatting = ref(false);
    const trimming = ref(false);
    const chatInput = ref("new question");
    const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestUserText = ref("");
    const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const chatErrorText = ref("");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);
    const onReloadMessages = vi.fn(async () => {});

    type ChannelLike = {
      emit: (event: AssistantDeltaEvent) => void;
    };
    let boundChannel: ChannelLike | null = null;

    const flow = useChatFlow({
      chatting,
      trimming,
      getConversationId: () => "conversation-1",
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      chatInput,
      clipboardImages,
      latestUserText,
      latestUserImages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      chatErrorText,
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      t: (key) => key,
      formatRequestFailed: (error) => String(error),
      removeBinaryPlaceholders: (text) => text,
      invokeSendChatMessage: vi.fn(async () => ({
        accepted: true,
        duplicate: false,
        eventId: "event-1",
        conversationId: "conversation-1",
        traceId: "trace-1",
        ingress: "accepted",
      })),
      invokeBindActiveChatViewStream: vi.fn(async ({ onDelta }) => {
        boundChannel = onDelta as unknown as ChannelLike;
      }),
      onReloadMessages,
    });

    await flow.bindActiveConversationStream("conversation-1");
    await flow.sendChat();
    boundChannel!.emit({
      kind: "activity_reasoning_delta",
      delta: "思维链1",
      streamCache: {
        assistantText: "",
        streamBlocks: [{ reasoning: "思维链1" }],
      },
    });
    await flushAsyncSteps();

    await flow.handleExternalAssistantDelta({
      conversationId: "conversation-1",
      event: {
        kind: "tool_status",
        toolName: "operate",
        toolCallId: "tool-1",
        toolStatus: "running",
        toolArgs: "{\"action\":\"wait\"}",
        message: "正在执行 operate",
        streamCache: {
          assistantText: "",
          streamBlocks: [],
        },
      },
    });
    await flushAsyncSteps();

    const draft = allMessages.value.find((message) => String(message.id || "").startsWith("__draft_assistant__:"));
    expect(toolStatusText.value).toBe("正在执行 operate");
    expect(toolStatusState.value).toBe("running");
    expect((draft?.providerMeta as Record<string, unknown> | undefined)?._streamBlocks).toEqual([{
      reasoning: "思维链1",
      text: "",
      tools: [],
    }]);
    expect(projectMessageForDisplay(draft as ChatMessage).activityItems.map((item) => item.text)).toEqual([
      "思维链1",
    ]);
  });
});

describe("useChatRuntime force archive conversation sync", () => {
  beforeEach(() => {
    hoisted.invokeTauriMock.mockReset();
  });

  it("updates current conversation id from trim_current_conversation before reload messages", async () => {
    const statusList: string[] = [];
    const errorList: string[] = [];
    const currentConversationId = ref("conv-old");
    const allMessages = shallowRef<ChatMessage[]>([]);
    const visibleTurnCount = ref(1);

    hoisted.invokeTauriMock.mockImplementation(async (command: string, payload?: unknown) => {
      if (command === "trim_current_conversation") {
        return {
          archived: true,
          archiveId: "archive-1",
          activeConversationId: "conv-new",
          summary: "ok",
          mergedMemories: 2,
        };
      }
      if (command === "get_foreground_conversation_light_snapshot") {
        const input = (payload as { input?: { conversationId?: string | null } } | undefined)?.input;
        return {
          messages: [
            textMessage(
              "a1",
              "assistant",
              `conversation:${String(input?.conversationId || "")}`,
            ),
          ],
        };
      }
      throw new Error(`unexpected invoke command: ${command}`);
    });

    const runtime = useChatRuntime({
      t: (key) => key,
      setStatus: (text) => statusList.push(text),
      setStatusError: (key, error) => errorList.push(`${key}:${String(error)}`),
      setChatError: () => {},
      activeChatApiConfigId: ref("api-1"),
      assistantDepartmentAgentId: ref("agent-1"),
      currentConversationId,
      chatting: ref(false),
      trimming: ref(false),
      compactingConversation: ref(false),
      allMessages,
      visibleMessageBlockCount: visibleTurnCount,
      perfNow: () => Date.now(),
      perfLog: () => {},
      perfDebug: false,
    });

    await runtime.trimNow();

    expect(currentConversationId.value).toBe("conv-new");
    expect(allMessages.value).toHaveLength(1);
    expect(allMessages.value[0].parts?.[0]).toEqual({
      type: "text",
      text: "conversation:conv-new",
    });
    expect(errorList).toEqual([]);
    expect(statusList.length).toBeGreaterThan(0);
  });
});
