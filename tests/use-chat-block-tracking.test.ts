import { describe, expect, it } from "vitest";
import { ref } from "vue";
import type { ChatMessageBlock } from "../src/types/app";
import type { ChatRenderItem } from "../src/features/chat/utils/chat-render";
import { useChatBlockTracking } from "../src/features/chat/composables/use-chat-block-tracking";

function block(overrides: Partial<ChatMessageBlock> = {}): ChatMessageBlock {
  return {
    id: "block-1",
    role: "assistant",
    text: "",
    images: [],
    audios: [],
    attachmentFiles: [],
    toolCallCount: 0,
    lastToolName: "",
    toolCalls: [],
    activityItems: [],
    activityReasoningCharCount: 0,
    activityToolCountsByName: {},
    activityRunning: false,
    activityStatus: "idle",
    ...overrides,
  };
}

describe("useChatBlockTracking", () => {
  it("uses plan started divider as elastic anchor when user and summary anchors are absent", () => {
    const planBlock = block({ id: "plan-divider", dividerKind: "plan_started" });
    const messageBlocks = ref<ChatMessageBlock[]>([planBlock]);
    const chatRenderItems = ref<ChatRenderItem[]>([
      { kind: "plan_started", id: "plan-started-plan-divider", renderId: "plan-divider", block: planBlock, blockIndex: 0 },
    ]);

    const tracking = useChatBlockTracking(messageBlocks, chatRenderItems);

    expect(tracking.latestOwnElasticItemId.value).toBe("plan-started-plan-divider");
  });

  it("does not fall back to the last ordinary visible message", () => {
    const assistantBlock = block({ id: "assistant-1", role: "assistant", text: "回复" });
    const messageBlocks = ref<ChatMessageBlock[]>([assistantBlock]);
    const chatRenderItems = ref<ChatRenderItem[]>([
      { kind: "message", id: "message-assistant-1", renderId: "assistant-1", block: assistantBlock, blockIndex: 0, compactWithPrevious: false },
    ]);

    const tracking = useChatBlockTracking(messageBlocks, chatRenderItems);

    expect(tracking.latestOwnElasticItemId.value).toBe("");
  });
});
