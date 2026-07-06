import { ref } from "vue";
import { describe, expect, it } from "vitest";
import { useChatFlowSendInput } from "./use-chat-flow-send-input";
import { useChatFlowSendPayloads } from "./use-chat-flow-send-payloads";

describe("useChatFlowSendInput", () => {
  it("keeps image attachment metadata single when the same file is also an image payload", () => {
    const queuedAttachmentNotices = ref([
      {
        id: "downloads/source.png::image/png",
        fileName: "source.png",
        relativePath: "downloads/source.png",
        mime: "image/png",
      },
    ]);
    const clipboardImages = ref([
      {
        mime: "image/png",
        bytesBase64: "aW1n",
        savedPath: "downloads/source.png",
      },
    ]);
    const payloads = useChatFlowSendPayloads({ queuedAttachmentNotices });
    const sendInput = useChatFlowSendInput({
      chatInput: ref("看图"),
      clipboardImages,
      queuedAttachmentNotices,
      selectedMentions: ref([]),
      latestUserText: ref(""),
      latestUserImages: ref([]),
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      getConversationId: () => "conversation-1",
      buildQueuedAttachmentPayload: payloads.buildQueuedAttachmentPayload,
      buildImageAttachmentPayload: payloads.buildImageAttachmentPayload,
      mergeAttachmentPayloads: payloads.mergeAttachmentPayloads,
    });

    const prepared = sendInput.prepareSendInput();

    expect(prepared?.attachments).toEqual([
      {
        fileName: "source.png",
        relativePath: "downloads/source.png",
        mime: "image/png",
      },
    ]);
    expect(prepared?.sentImages).toHaveLength(1);
  });

  it("adds an attachment metadata fallback for image payloads that only have savedPath", () => {
    const queuedAttachmentNotices = ref<Array<{ id: string; fileName: string; relativePath: string; mime: string }>>([]);
    const clipboardImages = ref([
      {
        mime: "image/png",
        bytesBase64: "aW1n",
        savedPath: "downloads/pasted.png",
      },
    ]);
    const payloads = useChatFlowSendPayloads({ queuedAttachmentNotices });
    const sendInput = useChatFlowSendInput({
      chatInput: ref(""),
      clipboardImages,
      queuedAttachmentNotices,
      selectedMentions: ref([]),
      latestUserText: ref(""),
      latestUserImages: ref([]),
      getSession: () => ({ apiConfigId: "api-1", agentId: "agent-1" }),
      getConversationId: () => "conversation-1",
      buildQueuedAttachmentPayload: payloads.buildQueuedAttachmentPayload,
      buildImageAttachmentPayload: payloads.buildImageAttachmentPayload,
      mergeAttachmentPayloads: payloads.mergeAttachmentPayloads,
    });

    const prepared = sendInput.prepareSendInput();

    expect(prepared?.attachments).toEqual([
      {
        fileName: "pasted.png",
        relativePath: "downloads/pasted.png",
        mime: "image/png",
      },
    ]);
    expect(prepared?.sentImages).toHaveLength(1);
  });
});
