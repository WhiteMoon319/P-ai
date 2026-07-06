import type { Ref } from "vue";

type QueuedAttachmentNotice = {
  id: string;
  fileName: string;
  relativePath: string;
  mime: string;
};

type ImageAttachment = {
  mime: string;
  bytesBase64: string;
  savedPath?: string;
};

export type AttachmentPayload = {
  fileName: string;
  relativePath: string;
  mime: string;
};

type UseChatFlowSendPayloadsOptions = {
  queuedAttachmentNotices?: Ref<QueuedAttachmentNotice[]>;
};

export function useChatFlowSendPayloads(options: UseChatFlowSendPayloadsOptions) {
  function attachmentPayloadKey(item: AttachmentPayload): string {
    return `${item.relativePath.replace(/\\/g, "/").toLowerCase()}::${item.mime.toLowerCase()}`;
  }

  function mergeAttachmentPayloads(
    primary: AttachmentPayload[],
    fallback: AttachmentPayload[] = [],
  ): AttachmentPayload[] {
    const merged = new Map<string, AttachmentPayload>();
    for (const item of [...primary, ...fallback]) {
      const fileName = String(item.fileName || "").trim();
      const relativePath = String(item.relativePath || "").trim().replace(/\\/g, "/");
      const mime = String(item.mime || "").trim();
      if (!fileName || !relativePath) continue;
      const normalized = { fileName, relativePath, mime };
      const key = attachmentPayloadKey(normalized);
      if (merged.has(key)) continue;
      merged.set(key, normalized);
    }
    return Array.from(merged.values());
  }

  function buildQueuedAttachmentPayload(): AttachmentPayload[] {
    const list = options.queuedAttachmentNotices?.value || [];
    if (list.length === 0) return [];
    const payloads = list
      .map((item) => {
        const fileName = String(item.fileName || "").trim();
        const relativePath = String(item.relativePath || "").trim().replace(/\\/g, "/");
        const mime = String(item.mime || "").trim();
        if (!fileName || !relativePath) return null;
        return { fileName, relativePath, mime };
      })
      .filter((value): value is AttachmentPayload => !!value);
    return mergeAttachmentPayloads(payloads);
  }

  function buildImageAttachmentPayload(images: ImageAttachment[]): AttachmentPayload[] {
    const payloads: AttachmentPayload[] = [];
    for (const image of images) {
      const rawPath = String(image.savedPath || "").trim();
      if (!rawPath) continue;
      const relativePath = rawPath.replace(/\\/g, "/");
      if (!relativePath) continue;
      const fileName = relativePath.split("/").pop() || "attachment";
      const mime = String(image.mime || "").trim();
      payloads.push({ fileName, relativePath, mime });
    }
    return mergeAttachmentPayloads(payloads);
  }

  return {
    buildQueuedAttachmentPayload,
    buildImageAttachmentPayload,
    mergeAttachmentPayloads,
  };
}
