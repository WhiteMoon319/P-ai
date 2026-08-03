import { ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeTauriMock = vi.hoisted(() => vi.fn());

vi.mock("../../../services/tauri-api", () => ({
  invokeTauri: invokeTauriMock,
}));

import { useConversationMaintenanceDialog } from "./use-conversation-maintenance-dialog";

describe("useConversationMaintenanceDialog", () => {
  beforeEach(() => {
    invokeTauriMock.mockReset();
  });

  it("两端共用同一块页预览和压缩执行入口，占用率直接复用 chatUsagePercent 数据源", async () => {
    const messages = Array.from({ length: 10 }, (_, index) => ({
      id: `message-${index}`,
      role: index % 2 === 0 ? "user" : "assistant",
      parts: [{ type: "text", text: `text-${index}` }],
      providerMeta: undefined,
    }));
    invokeTauriMock.mockResolvedValue({ selectedBlockId: 1, messages });
    const trimCompactNow = vi.fn(async () => {});
    const trimNow = vi.fn(async () => {});
    const deleteConversation = vi.fn(async () => {});
    const flow = useConversationMaintenanceDialog({
      t: (key) => key,
      currentConversationId: ref("conversation-a"),
      conversationSummaries: ref([{
        conversationId: "conversation-a",
        messageCount: 10,
        bodyMessageCount: 10,
        hasAssistantReply: true,
        runtimeState: "idle",
      }]),
      chatUsagePercent: ref(20),
      trimCompactNow,
      trimNow,
      deleteConversation,
      setStatus: vi.fn(),
      setStatusError: vi.fn(),
    });

    await flow.openTrimActionDialog();

    expect(invokeTauriMock).toHaveBeenCalledWith("conversation.blockPage", {
      input: { conversationId: "conversation-a" },
    });
    expect(flow.trimActionDialogOpen.value).toBe(true);
    expect(flow.trimPreview.value?.canArchive).toBe(true);
    expect(flow.trimCompactionPreview.value).toEqual(expect.objectContaining({
      canCompact: true,
      messageCount: 10,
      contextUsagePercent: 20,
    }));

    await flow.confirmTrimCompactionAction();
    expect(trimCompactNow).toHaveBeenCalledTimes(1);
    expect(flow.trimActionDialogOpen.value).toBe(false);
  });
});
