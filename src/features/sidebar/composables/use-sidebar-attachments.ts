import { computed, ref, type Ref } from "vue";
import { isAbsoluteLocalPath } from "../../chat/utils/local-link";
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
  return `${item.path.replace(/\\/g, "/").toLowerCase()}::${item.mime.toLowerCase()}`;
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
    path: item.path,
    mime: item.mime,
  })));

  async function appendClipboardImagesFromPaste(event: ClipboardEvent) {
    if (options.view.value !== "chat" || options.compacting.value) return;
    const files = pastedImageFiles(event);
    if (files.length === 0) return;
    event.preventDefault();
    for (const file of files) {
      try {
        const dataUrl = await readBlobAsDataUrl(file);
        const bytesBase64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
        if (!bytesBase64) continue;
        clipboardImages.value.push({
          mime: String(file.type || "image/png").trim() || "image/png",
          bytesBase64,
        });
      } catch (error) {
        console.warn("[侧边栏附件] 单张剪贴板图片读取失败，已跳过并继续", {
          fileName: String(file.name || "clipboard-image"),
          error: error instanceof Error ? error.message : String(error || "unknown"),
          stack: error instanceof Error ? error.stack : undefined,
        });
        options.errorText.value = String(error || options.t("sidebar.readClipboardImageFailed"));
      }
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
      const path = String(item.path || "").trim().replace(/\\/g, "/");
      const mime = String(item.mime || "").trim();
      if (!fileName || !isAbsoluteLocalPath(path)) continue;
      const normalized = { fileName, path, mime };
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
    for (const file of supported) {
      try {
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
        const path = savedPath.replace(/\\/g, "/");
        if (!isAbsoluteLocalPath(path)) {
          throw new Error(`附件未返回可用的绝对路径：${String(queued.fileName || file.name || "attachment")}`);
        }
        const fileName = String(queued.fileName || "").trim() || path.split("/").pop() || "attachment";
        const id = `${path}::${mime}`;
        if (!queuedAttachmentEntries.value.some((item) => item.id === id)) {
          queuedAttachmentEntries.value.push({
            id,
            fileName,
            path,
            mime,
            imageBytesBase64: mime.startsWith("image/") ? String(queued.bytesBase64 || "").trim() || undefined : undefined,
          });
        }
      } catch (error) {
        console.warn("[侧边栏附件] 单个附件处理失败，已跳过并继续", {
          fileName: String(file.name || "attachment"),
          error: error instanceof Error ? error.message : String(error || "unknown"),
          stack: error instanceof Error ? error.stack : undefined,
        });
        options.errorText.value = String(error || options.t("sidebar.readClipboardImageFailed"));
      }
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
    appendAttachmentFiles,
    appendClipboardImagesFromPaste,
    buildQueuedAttachmentPayload,
    handleAttachmentInputChange,
    pickAttachments,
    removeClipboardImage,
    removeQueuedAttachmentNotice,
  };
}
