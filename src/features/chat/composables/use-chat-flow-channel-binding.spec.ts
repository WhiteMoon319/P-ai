import { describe, expect, it, vi } from "vitest";
import type { TransportChannel } from "../../../services/tauri-api";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class<T> {
    onmessage: ((message: T) => void) | null = null;
  },
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(),
  emitTo: vi.fn(),
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ label: "chat" }),
}));

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: vi.fn(async () => () => {}) }),
}));

import { bindTransportConversationStream } from "../../../services/tauri-api";
import { useChatFlowChannelBinding } from "./use-chat-flow-channel-binding";
import type { AssistantDeltaEvent } from "./use-chat-flow-events";

describe("useChatFlowChannelBinding", () => {
  it("解绑调用尚未返回时也会立即丢弃旧 bound channel 事件", async () => {
    let boundChannel: TransportChannel<AssistantDeltaEvent> | null = null;
    let finishUnbind: () => void = () => {
      throw new Error("unbind resolver is not ready");
    };
    const handleStreamingEvent = vi.fn();
    const binding = useChatFlowChannelBinding({
      getConversationId: () => "conversation-a",
      invokeBindActiveChatViewStream: async ({ onDelta }) => {
        boundChannel = onDelta;
      },
      invokeUnbindActiveChatViewStream: () => new Promise<void>((resolve) => {
        finishUnbind = resolve;
      }),
      getRoundActiveGen: () => 1,
      getCurrentGeneration: () => 1,
      markHistoryFlushedReceived: vi.fn(),
      handleHistoryFlushed: vi.fn(async () => {}),
      handleStreamingEvent,
      formatRequestFailed: String,
      setChatErrorText: vi.fn(),
    });

    await binding.bindActiveConversationStream("conversation-a");
    const oldChannel = boundChannel as TransportChannel<AssistantDeltaEvent> | null;
    expect(oldChannel).not.toBeNull();

    const unbinding = binding.unbindActiveConversationStream();
    oldChannel?.onmessage?.({
      kind: "text_delta",
      delta: "late delta",
    } as AssistantDeltaEvent);

    expect(handleStreamingEvent).not.toHaveBeenCalled();
    finishUnbind();
    await unbinding;
  });

  it("同一窗口中的两个 flow 使用不同 bindingId 建立各自通道", async () => {
    const firstBind = vi.fn(async () => {});
    const secondBind = vi.fn(async () => {});
    const first = useChatFlowChannelBinding({
      getConversationId: () => "conversation-a",
      invokeBindActiveChatViewStream: firstBind,
      getRoundActiveGen: () => 1,
      getCurrentGeneration: () => 1,
      markHistoryFlushedReceived: vi.fn(),
      handleHistoryFlushed: vi.fn(async () => {}),
      handleStreamingEvent: vi.fn(),
      formatRequestFailed: String,
      setChatErrorText: vi.fn(),
    });
    const second = useChatFlowChannelBinding({
      getConversationId: () => "conversation-b",
      invokeBindActiveChatViewStream: secondBind,
      getRoundActiveGen: () => 1,
      getCurrentGeneration: () => 1,
      markHistoryFlushedReceived: vi.fn(),
      handleHistoryFlushed: vi.fn(async () => {}),
      handleStreamingEvent: vi.fn(),
      formatRequestFailed: String,
      setChatErrorText: vi.fn(),
    });

    await first.bindActiveConversationStream("conversation-a");
    await second.bindActiveConversationStream("conversation-b");

    expect(first.bindingId).not.toBe(second.bindingId);
    expect(firstBind).toHaveBeenCalledWith(expect.objectContaining({
      bindingId: first.bindingId,
      conversationId: "conversation-a",
    }));
    expect(secondBind).toHaveBeenCalledWith(expect.objectContaining({
      bindingId: second.bindingId,
      conversationId: "conversation-b",
    }));
  });

  it("桌面 App 经统一适配器绑定真实 Channel 后仍能驱动同一状态机", async () => {
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: {
        __TAURI_INTERNALS__: { invoke: vi.fn() },
        parent: null,
      },
    });
    invokeMock.mockResolvedValue(undefined);
    const handleStreamingEvent = vi.fn();
    const binding = useChatFlowChannelBinding({
      getConversationId: () => "conversation-native-stream",
      invokeBindActiveChatViewStream: bindTransportConversationStream,
      getRoundActiveGen: () => 7,
      getCurrentGeneration: () => 7,
      markHistoryFlushedReceived: vi.fn(),
      handleHistoryFlushed: vi.fn(async () => {}),
      handleStreamingEvent,
      formatRequestFailed: String,
      setChatErrorText: vi.fn(),
    });

    await binding.bindActiveConversationStream("conversation-native-stream");

    const bindCall = invokeMock.mock.calls.find(([command]) => command === "bind_active_chat_view_stream");
    const channel = bindCall?.[1]?.onDelta as TransportChannel<AssistantDeltaEvent> | undefined;
    expect(channel).toBeDefined();
    channel?.onmessage?.({ kind: "text_delta", delta: "App 流式恢复" } as AssistantDeltaEvent);

    expect(handleStreamingEvent).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ kind: "text_delta", delta: "App 流式恢复" }),
    );
    Reflect.deleteProperty(globalThis, "window");
  });
});
