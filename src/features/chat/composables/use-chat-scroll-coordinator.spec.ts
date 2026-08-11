// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useChatScrollCoordinator } from "./use-chat-scroll-coordinator";

function makeCoordinator() {
  const currentChatConversationId = ref("conversation-a");
  return {
    currentChatConversationId,
    ...useChatScrollCoordinator({ currentChatConversationId }),
  };
}

beforeEach(() => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      setTimeout: (handler: () => void, timeoutMs: number) => setTimeout(handler, timeoutMs),
      clearTimeout: (timer: unknown) => clearTimeout(timer as ReturnType<typeof setTimeout>),
    },
  });
});

afterEach(() => {
  vi.useRealTimers();
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: undefined,
  });
});

describe("requestScrollToBottomAfterStreamSettle", () => {
  it("流式中切会话：登记后不立即滚动，流式稳定（settle）后滚到底", () => {
    vi.useFakeTimers();
    const coordinator = makeCoordinator();
    const before = coordinator.conversationScrollToBottomRequest.value;

    coordinator.requestScrollToBottomAfterStreamSettle("conversation-a");
    // 登记期间不滚动
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before);

    coordinator.settleStreamScrollAfterStable("conversation-a");
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before + 1);
    vi.useRealTimers();
  });

  it("稳定信号来自其他会话时不滚动", () => {
    vi.useFakeTimers();
    const coordinator = makeCoordinator();
    const before = coordinator.conversationScrollToBottomRequest.value;

    coordinator.requestScrollToBottomAfterStreamSettle("conversation-a");
    coordinator.settleStreamScrollAfterStable("conversation-b");
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before);
    vi.useRealTimers();
  });

  it("超时兜底：流式一直不落库时也会滚动", () => {
    vi.useFakeTimers();
    const coordinator = makeCoordinator();
    const before = coordinator.conversationScrollToBottomRequest.value;

    coordinator.requestScrollToBottomAfterStreamSettle("conversation-a", 5000);
    vi.advanceTimersByTime(4999);
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before);

    vi.advanceTimersByTime(1);
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before + 1);
    vi.useRealTimers();
  });

  it("默认超时 1 秒：不传 timeoutMs 时 1 秒后兜底滚动", () => {
    vi.useFakeTimers();
    const coordinator = makeCoordinator();
    const before = coordinator.conversationScrollToBottomRequest.value;

    coordinator.requestScrollToBottomAfterStreamSettle("conversation-a");
    vi.advanceTimersByTime(999);
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before);

    vi.advanceTimersByTime(1);
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before + 1);
    vi.useRealTimers();
  });

  it("稳定后清除登记：不会再次触发滚动", () => {
    vi.useFakeTimers();
    const coordinator = makeCoordinator();
    const before = coordinator.conversationScrollToBottomRequest.value;

    coordinator.requestScrollToBottomAfterStreamSettle("conversation-a", 5000);
    coordinator.settleStreamScrollAfterStable("conversation-a");
    vi.advanceTimersByTime(10000);
    expect(coordinator.conversationScrollToBottomRequest.value).toBe(before + 1);
    vi.useRealTimers();
  });
});
