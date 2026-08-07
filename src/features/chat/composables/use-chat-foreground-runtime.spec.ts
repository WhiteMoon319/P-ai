import { ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import foregroundRuntimeSource from "./use-chat-foreground-runtime.ts?raw";

const invokeTauriMock = vi.hoisted(() => vi.fn());

vi.mock("../../../services/tauri-api", () => ({
  invokeTauri: invokeTauriMock,
  restoreTransportAfterForegroundWake: vi.fn(async () => {}),
  setTransportChatViewActive: vi.fn(async () => {}),
}));

import { useChatForegroundRuntime } from "./use-chat-foreground-runtime";

function message(id: string, text: string) {
  return { id, role: "assistant", createdAt: "2026-07-31T00:00:00Z", parts: [{ type: "text", text }] } as any;
}

describe("useChatForegroundRuntime", () => {
  beforeEach(() => invokeTauriMock.mockReset());

  it("会话 freshness 变化时以正式单条消息覆盖同 ID 的半截内容", async () => {
    const allMessages = ref([message("assistant-b", "半截")]);
    const conversationId = ref("conversation-a");
    const applyRuntime = vi.fn();
    invokeTauriMock.mockImplementation((command: string) => {
      if (command === "conversation.freshnessSnapshot") {
        return Promise.resolve({ lastMessageId: "assistant-b", updatedAt: "2026-08-07T00:00:01Z" });
      }
      if (command === "conversation.runtimeSnapshot") return Promise.resolve({ runtimeState: "idle" });
      if (command === "conversation.messageById") return Promise.resolve(message("assistant-b", "完成态"));
      return Promise.resolve({});
    });
    const runtime = useChatForegroundRuntime({
      viewMode: ref("chat"),
      chatWindowActiveSynced: ref(null),
      currentChatConversationId: conversationId,
      chatting: ref(false),
      allMessages,
      getChatFlow: () => ({
        frontendRoundPhase: ref("idle"),
        probeBoundChannel: vi.fn(async () => true),
      }),
      applyConversationRuntimeStateUpdated: applyRuntime,
      syncUnarchivedConversationOverviewChangedSinceWatermark: vi.fn(async () => {}),
      switchUnarchivedConversation: vi.fn(async () => {}),
    });

    await runtime.recoverForegroundConversation("test");

    expect((allMessages.value[0].parts[0] as any).text).toBe("完成态");
    expect(invokeTauriMock).toHaveBeenCalledWith("conversation.messageById", {
      input: { conversationId: "conversation-a", messageId: "assistant-b" },
    });
  });

  it("会话 freshness 未变化时不会请求正式单条消息", async () => {
    invokeTauriMock.mockImplementation((command: string) => {
      if (command === "conversation.freshnessSnapshot") {
        return Promise.resolve({ lastMessageId: "assistant-a", updatedAt: "2026-08-07T00:00:00Z" });
      }
      if (command === "conversation.runtimeSnapshot") return Promise.resolve({ runtimeState: "idle" });
      return Promise.resolve({});
    });
    const runtime = useChatForegroundRuntime({
      viewMode: ref("chat"),
      chatWindowActiveSynced: ref(null),
      currentChatConversationId: ref("conversation-a"),
      chatting: ref(false),
      allMessages: ref([message("assistant-a", "正文")]),
      getChatFlow: () => ({ frontendRoundPhase: ref("idle"), probeBoundChannel: vi.fn(async () => true) }),
      applyConversationRuntimeStateUpdated: vi.fn(),
      syncUnarchivedConversationOverviewChangedSinceWatermark: vi.fn(async () => {}),
      switchUnarchivedConversation: vi.fn(async () => {}),
    });

    // 首次恢复：建立基线并完成对账
    await runtime.recoverForegroundConversation("test");
    expect(invokeTauriMock).toHaveBeenCalledWith("conversation.messageById", {
      input: { conversationId: "conversation-a", messageId: "assistant-a" },
    });
    invokeTauriMock.mockClear();

    // 再次恢复：指纹未变化，不再请求正式单条消息
    await runtime.recoverForegroundConversation("test");
    expect(invokeTauriMock.mock.calls.some(([command]) => command === "conversation.messageById")).toBe(false);
  });

  it("运行时不识别 Tauri 或 Web bridge，只使用统一传输门面", () => {
    expect(foregroundRuntimeSource).not.toMatch(/isTauriRuntimeAvailable|acquireVsCodeApi|__PAI_SIDEBAR_BRIDGE__/);
  });
});
