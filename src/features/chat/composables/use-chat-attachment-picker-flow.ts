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
    // Product rule: 流式回复进行中不锁定输入工具栏（与粘贴/拖拽图片一致），
    // 仅压缩整理（trimming）期间禁止新增附件。
    if (options.trimming.value) return;
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
