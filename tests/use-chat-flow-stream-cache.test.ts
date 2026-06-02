import { describe, expect, it } from "vitest";
import { ref } from "vue";
import {
  streamCacheHasVisibleProgress,
  useChatFlowStreamCache,
} from "../src/features/chat/composables/use-chat-flow-stream-cache";
import type { AssistantStreamBlock } from "../src/types/app";

describe("useChatFlowStreamCache stream block snapshots", () => {
  it("restores visible thinking progress from stream block runtime snapshots", () => {
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);

    const cache = useChatFlowStreamCache({
      getConversationId: () => "conversation-1",
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      getActiveActivationId: () => "request-1",
      getFrontendDispatchStartedAtMs: () => 100,
      getFrontendDispatchElapsedMs: () => 8,
      currentFrontendDispatchElapsedMs: () => 8,
      restoreFrontendDispatchTimerFromCache: () => {},
    });

    const reasoningBlock: AssistantStreamBlock = { reasoning: "R1" };

    cache.writeConversationStreamCacheSnapshot("conversation-1", {
      activationId: "request-1",
      requestId: "request-1",
      assistantText: "",
      toolStatusText: "",
      toolStatusState: "",
      streamBlocks: [reasoningBlock],
    });

    expect(streamCacheHasVisibleProgress(cache.readConversationStreamCache("conversation-1"))).toBe(true);
    expect(cache.applyConversationStreamCacheToDisplay("conversation-1")).toBe(true);
    expect(streamBlocks.value).toEqual([{ reasoning: "R1", text: "", tools: [] }]);
    expect(latestAssistantText.value).toBe("");
  });

  it("can apply active stream snapshots after channel and generation already matched", () => {
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);

    const cache = useChatFlowStreamCache({
      getConversationId: () => "conversation-1",
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      getActiveActivationId: () => "foreground-activation",
      getFrontendDispatchStartedAtMs: () => 100,
      getFrontendDispatchElapsedMs: () => 8,
      currentFrontendDispatchElapsedMs: () => 8,
      restoreFrontendDispatchTimerFromCache: () => {},
    });

    expect(cache.applyConversationStreamCacheSnapshotToDisplay("conversation-1", {
      activationId: "backend-activation",
      requestId: "backend-activation",
      streamBlocks: [{ reasoning: "R2" }],
    })).toBe(false);
    expect(streamBlocks.value).toEqual([]);

    expect(cache.applyConversationStreamCacheSnapshotToDisplay("conversation-1", {
      activationId: "backend-activation",
      requestId: "backend-activation",
      streamBlocks: [{ reasoning: "R2" }],
    }, { ignoreActivationId: true })).toBe(true);
    expect(streamBlocks.value).toEqual([{ reasoning: "R2", text: "", tools: [] }]);
  });
});
