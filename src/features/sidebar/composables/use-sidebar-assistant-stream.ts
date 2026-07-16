import { computed, ref, type Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import {
  applyAssistantToolEventToStreamBlocks,
  appendTextDeltaToStreamBlocks,
  assistantTextFromStreamBlocks,
  normalizeAssistantStreamBlocks,
} from "../../../utils/chat-message-semantics";
import type {
  SidebarAssistantDeltaPayload,
  SidebarConversationRuntimePayload,
  SidebarStreamCachePayload,
} from "../sidebar-app-types";

type UseSidebarAssistantStreamOptions = {
  messages: Ref<ChatMessage[]>;
  activeAgentId: Ref<string>;
};

function normalizeToolStatusState(value: unknown): "running" | "done" | "failed" | "" {
  const status = String(value || "").trim();
  return status === "running" || status === "done" || status === "failed" ? status : "";
}

export function useSidebarAssistantStream(options: UseSidebarAssistantStreamOptions) {
  const toolStatusText = ref("");
  const toolStatusState = ref<"running" | "done" | "failed" | "">("");
  const streamingAssistantMessageId = ref("");
  const streamActivationId = ref("");
  const streamRequestId = ref("");
  const streamRevision = ref("");

  const activeMessage = computed(() => options.messages.value.find((message) =>
    String(message.id || "").trim() === streamingAssistantMessageId.value
  ));
  const activeMessageBlocks = computed(() => normalizeAssistantStreamBlocks(activeMessage.value?.contentBlocks));
  const activeMessageText = computed(() => assistantTextFromStreamBlocks(activeMessageBlocks.value));

  function clearStreamingState() {
    toolStatusText.value = "";
    toolStatusState.value = "";
    streamingAssistantMessageId.value = "";
    streamActivationId.value = "";
    streamRequestId.value = "";
    streamRevision.value = "";
  }

  function writeStreamCacheToMessage(cache: SidebarStreamCachePayload) {
    const messageId = String(cache.persistedAssistantMessageId || streamingAssistantMessageId.value || "").trim();
    if (!messageId) return;
    streamingAssistantMessageId.value = messageId;
    if (String(cache.activationId || "").trim()) {
      streamActivationId.value = String(cache.activationId || "").trim();
    }
    if (String(cache.requestId || "").trim()) {
      streamRequestId.value = String(cache.requestId || "").trim();
    }
    if (String(cache.updatedAt || "").trim()) {
      streamRevision.value = String(cache.updatedAt || "").trim();
    }
    const blocks = normalizeAssistantStreamBlocks(cache.streamBlocks);
    const index = options.messages.value.findIndex((message) => String(message.id || "").trim() === messageId);
    const previous = index >= 0 ? options.messages.value[index] : undefined;
    const previousMeta = (previous?.providerMeta || {}) as Record<string, unknown>;
    const message: ChatMessage = {
      ...(previous || {
        id: messageId,
        role: "assistant",
        createdAt: new Date().toISOString(),
        speakerAgentId: options.activeAgentId.value || undefined,
        parts: [{ type: "text", text: "" }],
      }),
      contentBlocks: blocks,
      providerMeta: {
        ...previousMeta,
        _streaming: true,
        _toolStatusText: String(cache.toolStatusText || ""),
        _toolStatusState: normalizeToolStatusState(cache.toolStatusState),
      },
    };
    options.messages.value = index >= 0
      ? options.messages.value.map((item, itemIndex) => itemIndex === index ? message : item)
      : [...options.messages.value, message];
  }

  function startStreamingMessage(messageId: string) {
    clearStreamingState();
    streamingAssistantMessageId.value = String(messageId || "").trim();
    writeStreamCacheToMessage({
      persistedAssistantMessageId: streamingAssistantMessageId.value,
      streamBlocks: [],
      toolStatusText: "",
      toolStatusState: "",
    });
  }

  function finishStreamingMessage(messageId: string) {
    const normalizedId = String(messageId || "").trim();
    if (!normalizedId) return;
    options.messages.value = options.messages.value.map((message) => {
      if (String(message.id || "").trim() !== normalizedId) return message;
      const meta = { ...((message.providerMeta || {}) as Record<string, unknown>) };
      delete meta._streaming;
      delete meta._preStreamingStatusText;
      delete meta._toolStatusText;
      delete meta._toolStatusState;
      return { ...message, providerMeta: meta };
    });
  }

  function applyRuntimeStreamCache(runtime: SidebarConversationRuntimePayload | null | undefined) {
    const cache = runtime?.streamCache;
    if (!cache) return;
    writeStreamCacheToMessage(cache);
    toolStatusText.value = String(cache.toolStatusText || "");
    toolStatusState.value = normalizeToolStatusState(cache.toolStatusState);
  }

  function applyAssistantToolStatusEvent(event: NonNullable<SidebarAssistantDeltaPayload["event"]>) {
    toolStatusText.value = String(event.message || "");
    toolStatusState.value = normalizeToolStatusState(event.toolStatus);
  }

  function applyAssistantToolEvent(message: string) {
    const blocks = applyAssistantToolEventToStreamBlocks(activeMessage.value?.contentBlocks, message);
    writeStreamCacheToMessage({
      persistedAssistantMessageId: streamingAssistantMessageId.value,
      streamBlocks: blocks,
      toolStatusText: toolStatusText.value,
      toolStatusState: toolStatusState.value,
    });
  }

  function appendAssistantTextDelta(delta: string) {
    const blocks = appendTextDeltaToStreamBlocks(activeMessage.value?.contentBlocks, delta);
    writeStreamCacheToMessage({
      persistedAssistantMessageId: streamingAssistantMessageId.value,
      streamBlocks: blocks,
      toolStatusText: toolStatusText.value,
      toolStatusState: toolStatusState.value,
    });
  }

  return {
    activeMessageBlocks,
    activeMessageText,
    streamingAssistantMessageId,
    streamActivationId,
    streamRequestId,
    streamRevision,
    toolStatusState,
    toolStatusText,
    appendAssistantTextDelta,
    applyAssistantToolEvent,
    applyAssistantToolStatusEvent,
    applyRuntimeStreamCache,
    clearStreamingState,
    finishStreamingMessage,
    startStreamingMessage,
    writeStreamCacheToMessage,
  };
}
