import { computed, ref, type Ref } from "vue";
import { attachmentPreviewBase64, type AttachmentReceipt } from "../../../services/attachment-transfer";
import { isAbsoluteLocalPath } from "../../chat/utils/local-link";
import type {
  SidebarAttachmentPayload,
  SidebarClipboardImage,
  SidebarQueuedAttachmentEntry,
  SidebarQueuedAttachmentNotice,
} from "../sidebar-app-types";

type UseSidebarAttachmentsOptions = {
  view: Ref<string>;
  busy: Ref<boolean>;
  compacting: Ref<boolean>;
  errorText: Ref<string>;
  t: (key: string) => string;
  uploadAttachment: (file: File) => Promise<AttachmentReceipt>;
};

function pastedFiles(event: ClipboardEvent): File[] {
  const data = event.clipboardData;
  if (!data) return [];
  const filesFromItems = data.items && data.items.length > 0
    ? Array.from(data.items)
      .filter((item) => item.kind === "file")
      .map((item) => item.getAsFile())
      .filter((file): file is File => !!file)
    : [];
  if (filesFromItems.length > 0) return filesFromItems;
  return data.files
    ? Array.from(data.files)
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
    .filter((item) => item.mime.startsWith("image/") && (
      !!String(item.imageBytesBase64 || "").trim() || !!String(item.previewDataUrl || "").trim()
    ))
    .map((item) => ({
      mime: item.mime,
      bytesBase64: String(item.imageBytesBase64 || "").trim(),
      previewDataUrl: String(item.previewDataUrl || "").trim() || undefined,
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
    const files = pastedFiles(event);
    if (files.length === 0) return;
    event.preventDefault();
    await appendAttachmentFiles(files);
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
      .filter((item) => item.mime.startsWith("image/") && (
        !!String(item.imageBytesBase64 || "").trim() || !!String(item.previewDataUrl || "").trim()
      ))
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
    if (files.length === 0) return;
    for (const file of files) {
      try {
        const queued = await options.uploadAttachment(file);
        const mime = String(queued.mime || "").trim().toLowerCase();
        const path = String(queued.path || "").trim().replace(/\\/g, "/");
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
            imageBytesBase64: mime.startsWith("image/") ? attachmentPreviewBase64(queued) || undefined : undefined,
            previewDataUrl: mime.startsWith("image/") ? String(queued.previewDataUrl || "").trim() || undefined : undefined,
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
