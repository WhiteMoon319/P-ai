import { nextTick, ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useChatToolReview } from "./use-chat-tool-review";

vi.mock("../../../services/tauri-api", () => ({
  invokeTauri: vi.fn(),
}));

import { invokeTauri } from "../../../services/tauri-api";

const mockInvoke = vi.mocked(invokeTauri);

function createOptions() {
  const activeConversationId = ref("");
  const refreshTick = ref(0);
  const activeTab = ref("delegates");
  return {
    activeConversationId,
    refreshTick,
    activeTab,
    t: (key: string) => key,
  };
}

async function flushWatchers() {
  await nextTick();
  await Promise.resolve();
}

describe("useChatToolReview 批列表懒加载", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue({ batches: [], reports: [] });
  });

  it("面板未打开时切换会话不拉批列表", async () => {
    const options = createOptions();
    useChatToolReview(options);
    await flushWatchers();

    options.activeConversationId.value = "conversation-a";
    await flushWatchers();

    expect(mockInvoke).not.toHaveBeenCalledWith("list_tool_review_batches", expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith("list_tool_review_reports", expect.anything());
  });

  it("面板打开但标签不是 tools 时切换会话不拉批列表", async () => {
    const options = createOptions();
    const { toggleToolReviewPanel } = useChatToolReview(options);
    toggleToolReviewPanel();
    await flushWatchers();

    options.activeConversationId.value = "conversation-a";
    await flushWatchers();

    expect(mockInvoke).not.toHaveBeenCalledWith("list_tool_review_batches", expect.anything());
  });

  it("面板打开且 tools 标签激活时切换会话才拉批列表", async () => {
    const options = createOptions();
    const { toggleToolReviewPanel } = useChatToolReview(options);
    toggleToolReviewPanel();
    options.activeTab.value = "tools";
    await flushWatchers();

    options.activeConversationId.value = "conversation-a";
    await flushWatchers();

    expect(mockInvoke).toHaveBeenCalledWith("list_tool_review_batches", expect.objectContaining({
      conversationId: "conversation-a",
    }));
  });

  it("从其他标签切到 tools 标签时立即拉批列表", async () => {
    const options = createOptions();
    const { toggleToolReviewPanel } = useChatToolReview(options);
    toggleToolReviewPanel();
    options.activeConversationId.value = "conversation-a";
    await flushWatchers();

    mockInvoke.mockClear();
    options.activeTab.value = "tools";
    await flushWatchers();

    expect(mockInvoke).toHaveBeenCalledWith("list_tool_review_batches", expect.objectContaining({
      conversationId: "conversation-a",
    }));
  });
});
