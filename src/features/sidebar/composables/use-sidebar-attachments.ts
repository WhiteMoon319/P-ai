import { computed, ref, type Ref } from "vue";
import type {
  SidebarAttachmentPayload,
  SidebarClipboardImage,
  SidebarQueuedAttachmentEntry,
  SidebarQueuedAttachmentNotice,
} from "../sidebar-app-types";

type QueueAttachmentResult = {
  mime: string;
  fileName: string;
  savedPath: string;
  attachAsMedia: boolean;
  bytesBase64?: string | null;
};

type UseSidebarAttachmentsOptions = {
  view: Ref<string>;
  busy: Ref<boolean>;
  compacting: Ref<boolean>;
  errorText: Ref<string>;
  t: (key: string) => string;
  queueAttachment: (input: {
    fileName: string;
    mime: string;
    bytesBase64: string;
  }) => Promise<QueueAttachmentResult>;
};

function readBlobAsDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ""));
    reader.onerror = () => reject(reader.error || new Error("读取剪贴板图片失败"));
    reader.readAsDataURL(blob);
  });
}

function pastedImageFiles(event: ClipboardEvent): File[] {
  const data = event.clipboardData;
  if (!data) return [];
  const filesFromItems = data.items && data.items.length > 0
    ? Array.from(data.items)
      .filter((item) => item.kind === "file" && item.type.toLowerCase().startsWith("image/"))
      .map((item) => item.getAsFile())
      .filter((file): file is File => !!file)
    : [];
  if (filesFromItems.length > 0) return filesFromItems;
  return data.files
    ? Array.from(data.files).filter((file) => String(file.type || "").toLowerCase().startsWith("image/"))
    : [];
}

function attachmentPayloadKey(item: SidebarAttachmentPayload): string {
  return `${item.relativePath.replace(/\\/g, "/").toLowerCase()}::${item.mime.toLowerCase()}`;
}

export function useSidebarAttachments(options: UseSidebarAttachmentsOptions) {
  const clipboardImages = ref<SidebarClipboardImage[]>([]);
  const queuedAttachmentEntries = ref<SidebarQueuedAttachmentEntry[]>([]);
  const attachmentInputRef = ref<HTMLInputElement | null>(null);

  const attachmentBackedClipboardImages = computed<SidebarClipboardImage[]>(() => queuedAttachmentEntries.value
    .filter((item) => item.mime.startsWith("image/") && !!String(item.imageBytesBase64 || "").trim())
    .map((item) => ({
      mime: item.mime,
      bytesBase64: String(item.imageBytesBase64 || "").trim(),
    })));
  const composerClipboardImages = computed<SidebarClipboardImage[]>(() => [
    ...clipboardImages.value,
    ...attachmentBackedClipboardImages.value,
  ]);
  const queuedAttachmentNotices = computed<SidebarQueuedAttachmentNotice[]>(() => queuedAttachmentEntries.value.map((item) => ({
    id: item.id,
    fileName: item.fileName,
    relativePath: item.relativePath,
    mime: item.mime,
  })));

  async function appendClipboardImagesFromPaste(event: ClipboardEvent) {
    if (options.view.value !== "chat" || options.compacting.value) return;
    const files = pastedImageFiles(event);
    if (files.length === 0) return;
    event.preventDefault();
    try {
      for (const file of files) {
        const dataUrl = await readBlobAsDataUrl(file);
        const bytesBase64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
        if (!bytesBase64) continue;
        clipboardImages.value.push({
          mime: String(file.type || "image/png").trim() || "image/png",
          bytesBase64,
        });
      }
    } catch (error) {
      options.errorText.value = String(error || options.t("sidebar.readClipboardImageFailed"));
    }
  }

  function removeClipboardImage(index: number) {
    if (index < 0) return;
    if (index < clipboardImages.value.length) {
      clipboardImages.value.splice(index, 1);
      return;
    }
    const attachmentImageIndex = index - clipboardImages.value.length;
    if (attachmentImageIndex < 0) return;
    const attachmentImageIds = queuedAttachmentEntries.value
      .filter((item) => item.mime.startsWith("image/") && !!String(item.imageBytesBase64 || "").trim())
      .map((item) => item.id);
    const targetId = attachmentImageIds[attachmentImageIndex];
    if (!targetId) return;
    queuedAttachmentEntries.value = queuedAttachmentEntries.value.filter((item) => item.id !== targetId);
  }

  function removeQueuedAttachmentNotice(index: number) {
    if (index < 0 || index >= queuedAttachmentEntries.value.length) return;
    queuedAttachmentEntries.value.splice(index, 1);
  }

  function buildQueuedAttachmentPayload(): SidebarAttachmentPayload[] {
    const merged = new Map<string, SidebarAttachmentPayload>();
    for (const item of queuedAttachmentEntries.value) {
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

  function pickAttachments() {
    if (options.busy.value || options.compacting.value) return;
    if (!attachmentInputRef.value) return;
    attachmentInputRef.value.value = "";
    attachmentInputRef.value.click();
  }

  async function appendAttachmentFiles(files: File[]) {
    const supported = files.filter((file) => {
      const mime = String(file.type || "").toLowerCase();
      return mime.startsWith("image/") || mime === "application/pdf";
    });
    if (supported.length === 0) return;
    try {
      for (const file of supported) {
        const dataUrl = await readBlobAsDataUrl(file);
        const bytesBase64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
        if (!bytesBase64) continue;
        const queued = await options.queueAttachment({
          fileName: String(file.name || "").trim() || "attachment",
          mime: String(file.type || "").trim() || "application/octet-stream",
          bytesBase64,
        });
        const mime = String(queued.mime || "").trim().toLowerCase();
        const savedPath = String(queued.savedPath || "").trim();
        const relativePath = savedPath.replace(/\\/g, "/").replace(/^.*\/downloads\//, "downloads/");
        const fileName = String(queued.fileName || "").trim() || relativePath.split("/").pop() || "attachment";
        const id = `${relativePath || fileName}::${mime}`;
        if (!queuedAttachmentEntries.value.some((item) => item.id === id)) {
          queuedAttachmentEntries.value.push({
            id,
            fileName,
            relativePath: relativePath || savedPath || fileName,
            mime,
            imageBytesBase64: mime.startsWith("image/") ? String(queued.bytesBase64 || "").trim() || undefined : undefined,
          });
        }
      }
    } catch (error) {
      options.errorText.value = String(error || options.t("sidebar.readClipboardImageFailed"));
    }
  }

  function handleAttachmentInputChange(event: Event) {
    const target = event.target as HTMLInputElement | null;
    const files = target?.files ? Array.from(target.files) : [];
    void appendAttachmentFiles(files);
  }

  return {
    attachmentInputRef,
    clipboardImages,
    composerClipboardImages,
    queuedAttachmentEntries,
    queuedAttachmentNotices,
    appendClipboardImagesFromPaste,
    buildQueuedAttachmentPayload,
    handleAttachmentInputChange,
    pickAttachments,
    removeClipboardImage,
    removeQueuedAttachmentNotice,
  };
}
