import { ref } from "vue";
import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../../../types/app";
import { useSidebarAssistantStream } from "./use-sidebar-assistant-stream";

function createRuntime() {
  const messages = ref<ChatMessage[]>([]);
  const runtime = useSidebarAssistantStream({
    messages,
    activeAgentId: ref("agent-1"),
  });
  return { messages, runtime };
}

describe("useSidebarAssistantStream", () => {
  it("creates one formal assistant message and updates it by id", () => {
    const { messages, runtime } = createRuntime();

    runtime.startStreamingMessage("assistant-1");
    runtime.writeStreamCacheToMessage({
      persistedAssistantMessageId: "assistant-1",
      streamBlocks: [{ reasoning: "思考", text: "正文", tools: [] }],
    });

    expect(messages.value).toHaveLength(1);
    expect(messages.value[0].id).toBe("assistant-1");
    expect(messages.value[0].contentBlocks?.[0]?.reasoning).toBe("思考");
    expect(messages.value[0].contentBlocks?.[0]?.text).toBe("正文");
  });

  it("appends text to the canonical blocks without replacing the message", () => {
    const { messages, runtime } = createRuntime();
    runtime.startStreamingMessage("assistant-1");
    const originalMessage = messages.value[0];

    runtime.appendAssistantTextDelta("第一段");
    runtime.appendAssistantTextDelta("第二段");

    expect(messages.value).toHaveLength(1);
    expect(messages.value[0].id).toBe(originalMessage.id);
    expect(runtime.activeMessageText.value).toBe("第一段第二段");
  });

  it("tracks the latest backend stream revision for foreground recovery", () => {
    const { runtime } = createRuntime();
    runtime.startStreamingMessage("assistant-1");
    runtime.writeStreamCacheToMessage({
      persistedAssistantMessageId: "assistant-1",
      updatedAt: "2026-07-17T00:00:01Z",
      streamBlocks: [],
    });

    expect(runtime.streamRevision.value).toBe("2026-07-17T00:00:01Z");
  });

  it("tracks backend activation and request identity", () => {
    const { runtime } = createRuntime();
    runtime.startStreamingMessage("assistant-1");
    runtime.writeStreamCacheToMessage({
      persistedAssistantMessageId: "assistant-1",
      activationId: "activation-1",
      requestId: "request-1",
      streamBlocks: [],
    });

    expect(runtime.streamActivationId.value).toBe("activation-1");
    expect(runtime.streamRequestId.value).toBe("request-1");
  });

  it("finishes by clearing streaming metadata while preserving blocks", () => {
    const { messages, runtime } = createRuntime();
    runtime.startStreamingMessage("assistant-1");
    runtime.writeStreamCacheToMessage({
      persistedAssistantMessageId: "assistant-1",
      streamBlocks: [{ reasoning: "思考", text: "正文", tools: [] }],
      toolStatusText: "正在执行",
      toolStatusState: "running",
    });
    const blocksBefore = messages.value[0].contentBlocks;

    runtime.finishStreamingMessage("assistant-1");

    expect(messages.value[0].contentBlocks).toBe(blocksBefore);
    expect(messages.value[0].providerMeta?._streaming).toBeUndefined();
    expect(messages.value[0].providerMeta?._toolStatusText).toBeUndefined();
    expect(messages.value[0].providerMeta?._toolStatusState).toBeUndefined();
  });
});
