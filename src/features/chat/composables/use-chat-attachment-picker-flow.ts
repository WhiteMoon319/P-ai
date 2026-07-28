import type { Ref } from "vue";
import type { AttachmentReceipt } from "../../../services/attachment-transfer";
import { pickTransportAttachments } from "../../../services/tauri-api";

type QueuedAttachmentNotice = {
  id: string;
  fileName: string;
  path: string;
  mime: string;
};

type UseChatAttachmentPickerFlowOptions = {
  chatting: Ref<boolean>;
  trimming: Ref<boolean>;
  queuedAttachmentNotices: Ref<QueuedAttachmentNotice[]>;
  applyPickedAttachment: (receipt: AttachmentReceipt) => void;
  setStatusError: (key: string, error: unknown) => void;
};

export function useChatAttachmentPickerFlow(options: UseChatAttachmentPickerFlowOptions) {
  function removeQueuedAttachmentNotice(index: number) {
    if (index < 0 || index >= options.queuedAttachmentNotices.value.length) return;
    options.queuedAttachmentNotices.value.splice(index, 1);
  }

  async function pickChatAttachments() {
    if (options.chatting.value || options.trimming.value) return;
    try {
      const receipts = await pickTransportAttachments<AttachmentReceipt>({
        multiple: true,
        directory: false,
        title: "选择附件",
      });
      for (const receipt of receipts) options.applyPickedAttachment(receipt);
    } catch (error) {
      options.setStatusError("status.pasteImageReadFailed", error);
    }
  }

  return {
    removeQueuedAttachmentNotice,
    pickChatAttachments,
  };
}
