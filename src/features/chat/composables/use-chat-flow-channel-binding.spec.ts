import { describe, expect, it, vi } from "vitest";
import type { Channel } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class<T> {
    onmessage: ((message: T) => void) | null = null;
  },
}));

import { useChatFlowChannelBinding } from "./use-chat-flow-channel-binding";
import type { AssistantDeltaEvent } from "./use-chat-flow-events";

describe("useChatFlowChannelBinding", () => {
  it("解绑调用尚未返回时也会立即丢弃旧 bound channel 事件", async () => {
    let boundChannel: Channel<AssistantDeltaEvent> | null = null;
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
    const oldChannel = boundChannel as Channel<AssistantDeltaEvent> | null;
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
});
