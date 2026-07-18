import type { Ref } from "vue";

type QueuedAttachmentNotice = {
  id: string;
  fileName: string;
  path: string;
  mime: string;
};

type ImageAttachment = {
  mime: string;
  bytesBase64: string;
  savedPath?: string;
};

export type AttachmentPayload = {
  fileName: string;
  path: string;
  mime: string;
};

type UseChatFlowSendPayloadsOptions = {
  queuedAttachmentNotices?: Ref<QueuedAttachmentNotice[]>;
};

export function useChatFlowSendPayloads(options: UseChatFlowSendPayloadsOptions) {
  function attachmentPayloadKey(item: AttachmentPayload): string {
    return `${item.path.replace(/\\/g, "/").toLowerCase()}::${item.mime.toLowerCase()}`;
  }

  function mergeAttachmentPayloads(
    primary: AttachmentPayload[],
    fallback: AttachmentPayload[] = [],
  ): AttachmentPayload[] {
    const merged = new Map<string, AttachmentPayload>();
    for (const item of [...primary, ...fallback]) {
      const fileName = String(item.fileName || "").trim();
      const path = String(item.path || "").trim().replace(/\\/g, "/");
      const mime = String(item.mime || "").trim();
      if (!fileName || !path) continue;
      const normalized = { fileName, path, mime };
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
        const path = String(item.path || "").trim().replace(/\\/g, "/");
        const mime = String(item.mime || "").trim();
        if (!fileName || !path) return null;
        return { fileName, path, mime };
      })
      .filter((value): value is AttachmentPayload => !!value);
    return mergeAttachmentPayloads(payloads);
  }

  function buildImageAttachmentPayload(images: ImageAttachment[]): AttachmentPayload[] {
    const payloads: AttachmentPayload[] = [];
    for (const image of images) {
      const rawPath = String(image.savedPath || "").trim();
      if (!rawPath) continue;
      const path = rawPath.replace(/\\/g, "/");
      if (!path) continue;
      const fileName = path.split("/").pop() || "attachment";
      const mime = String(image.mime || "").trim();
      payloads.push({ fileName, path, mime });
    }
    return mergeAttachmentPayloads(payloads);
  }

  return {
    buildQueuedAttachmentPayload,
    buildImageAttachmentPayload,
    mergeAttachmentPayloads,
  };
}
