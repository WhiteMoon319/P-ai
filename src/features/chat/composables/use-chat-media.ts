import { ref, type ComputedRef, type Ref } from "vue";
import type { ApiConfigItem } from "../../../types/app";
import {
  attachmentPreviewBase64,
  ingestAttachment,
  textAttachmentFile,
  type AttachmentReceipt,
} from "../../../services/attachment-transfer";
import { useHotkeyRecordTest } from "../../shell/composables/use-hotkey-record-test";
import { isAbsoluteLocalPath } from "../utils/local-link";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

type UseChatMediaOptions = {
  t: TrFn;
  setStatus: (text: string) => void;
  setChatError: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  viewMode: Ref<"chat" | "archives" | "config">;
  chatting: Ref<boolean>;
  trimming: Ref<boolean>;
  isRecording: () => boolean;
  activeChatApiConfig: ComputedRef<ApiConfigItem | null>;
  hasVisionFallback: ComputedRef<boolean>;
  chatInput: Ref<string>;
  clipboardImages: Ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>;
  queuedAttachmentNotices: Ref<Array<{ id: string; fileName: string; path: string; mime: string }>>;
};

export function useChatMedia(options: UseChatMediaOptions) {
  const mediaDragActive = ref(false);
  let dragOverlayHideTimer: ReturnType<typeof setTimeout> | null = null;
  const hotkeyRecordTest = useHotkeyRecordTest({
    t: options.t,
    setStatus: options.setStatus,
    setStatusError: options.setStatusError,
    isBlocked: options.isRecording,
  });

  function canAcceptImage(apiConfig: ApiConfigItem): boolean {
    return !!apiConfig.enableImage || options.hasVisionFallback.value;
  }

  async function queueTextAttachment(fileName: string, text: string, mime = "text/markdown") {
    const normalizedText = String(text || "");
    if (!normalizedText.trim()) return;
    const queued = await ingestAttachment({
      kind: "browser-file",
      file: textAttachmentFile(fileName, normalizedText, mime),
    });
    applyQueuedAttachmentResult(queued, options.activeChatApiConfig.value || ({ enableImage: false } as ApiConfigItem));
  }

  function classifyFileMime(
    mime: string,
    apiConfig: ApiConfigItem,
  ): { kind: "image" | "pdf" | null; reason: "imageUnsupported" | null } {
    const normalized = (mime || "").trim().toLowerCase();
    if (normalized.startsWith("image/")) {
      return canAcceptImage(apiConfig)
        ? { kind: "image", reason: null }
        : { kind: null, reason: "imageUnsupported" };
    }
    if (normalized === "application/pdf") {
      // PDF 不再走多模态直发，统一入队为普通附件，交由后端阅读链路处理。
      return { kind: null, reason: null };
    }
    return { kind: null, reason: null };
  }

  function inferMimeFromFileName(name: string): string {
    const lower = (name || "").trim().toLowerCase();
    if (lower.endsWith(".pdf")) return "application/pdf";
    if (lower.endsWith(".png")) return "image/png";
    if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
    if (lower.endsWith(".gif")) return "image/gif";
    if (lower.endsWith(".webp")) return "image/webp";
    if (lower.endsWith(".heic")) return "image/heic";
    if (lower.endsWith(".heif")) return "image/heif";
    if (lower.endsWith(".svg")) return "image/svg+xml";
    return "";
  }

  function normalizeFileMime(file: File): string {
    const raw = (file.type || "").trim().toLowerCase();
    if (raw) return raw;
    return inferMimeFromFileName(file.name);
  }

  function hasFileTransferPayload(transfer: DataTransfer | null): boolean {
    if (!transfer) return false;
    const types = Array.from(transfer.types || []).map((value) => String(value || "").toLowerCase());
    if (types.includes("files")) return true;
    if (transfer.files && transfer.files.length > 0) return true;
    if (transfer.items && Array.from(transfer.items).some((item) => item.kind === "file")) return true;
    return false;
  }

  function collectPastedFiles(
    event: ClipboardEvent,
  ): Array<{ file: File; mime: string }> {
    const data = event.clipboardData;
    if (!data) return [];
    const items = data.items;
    const filesFromItems =
      items && items.length > 0
        ? Array.from(items)
            .filter((item) => item.kind === "file")
            .map((item) => item.getAsFile())
            .filter((file): file is File => !!file)
        : [];
    const filesFromList = data.files ? Array.from(data.files) : [];
    const sourceFiles = filesFromItems.length > 0 ? filesFromItems : filesFromList;
    if (sourceFiles.length === 0) return [];
    const files: Array<{ file: File; mime: string }> = [];
    for (const file of sourceFiles) {
      const mime = normalizeFileMime(file);
      files.push({ file, mime });
    }
    return files;
  }

  function collectDroppedFiles(
    event: DragEvent,
  ): Array<{ file: File; mime: string }> {
    const transfer = event.dataTransfer;
    if (!transfer) return [];
    const fromFiles = transfer.files ? Array.from(transfer.files) : [];
    const fromItems =
      transfer.items && transfer.items.length > 0
        ? Array.from(transfer.items)
            .filter((item) => item.kind === "file")
            .map((item) => item.getAsFile())
            .filter((file): file is File => !!file)
        : [];
    const files = fromFiles.length > 0 ? fromFiles : fromItems;
    if (files.length === 0) return [];
    const out: Array<{ file: File; mime: string }> = [];
    for (const file of files) {
      const mime = normalizeFileMime(file);
      out.push({ file, mime });
    }
    return out;
  }

  function applyQueuedAttachmentResult(queued: AttachmentReceipt, apiConfig: ApiConfigItem) {
    const mime = String(queued.mime || "").trim().toLowerCase();
    const classified = classifyFileMime(mime, apiConfig);
    const canAttachAsMedia = !!queued.attachAsMedia && !!classified.kind;
    const path = String(queued.path || "").trim().replace(/\\/g, "/");
    if (!isAbsoluteLocalPath(path)) {
      options.setChatError("附件保存未返回绝对路径，已跳过该附件。其他消息内容仍可继续发送。");
      return;
    }

    if (!canAttachAsMedia) {
      const fileName = String(queued.fileName || "").trim() || path.split("/").pop() || "attachment";
      const id = `${path}::${mime}`;
      if (!options.queuedAttachmentNotices.value.some((item) => item.id === id)) {
        options.queuedAttachmentNotices.value.push({
          id,
          fileName,
          path,
          mime,
        });
      }
      return;
    }

    const previewImage = {
      mime,
      bytesBase64: attachmentPreviewBase64(queued),
      savedPath: path,
      previewDataUrl: String(queued.previewDataUrl || "").trim() || undefined,
    };
    options.clipboardImages.value.push(previewImage);
  }

  async function queueInlineBrowserFile(file: File, _mime: string): Promise<AttachmentReceipt> {
    return await ingestAttachment({ kind: "browser-file", file });
  }

  function onPaste(event: ClipboardEvent) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    const collected = collectPastedFiles(event);
    if (collected.length > 0) {
      event.preventDefault();
      options.setChatError("");
      void (async () => {
        for (const item of collected) {
          try {
            const queued = await queueInlineBrowserFile(item.file, item.mime);
            applyQueuedAttachmentResult(queued, apiConfig);
          } catch (error) {
            options.setStatusError("status.pasteImageReadFailed", error);
          }
        }
      })();
      return;
    }

    const text = event.clipboardData?.getData("text/plain") || "";
    if (text && !options.chatInput.value.trim() && apiConfig.enableText) {
      event.preventDefault();
      options.chatInput.value = text;
      options.setChatError("");
      return;
    }

  }

  function onDragOver(event: DragEvent) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    if (!hasFileTransferPayload(event.dataTransfer)) return;
    event.preventDefault();
    event.dataTransfer!.dropEffect = "copy";
    mediaDragActive.value = true;
    if (dragOverlayHideTimer) {
      clearTimeout(dragOverlayHideTimer);
      dragOverlayHideTimer = null;
    }
    dragOverlayHideTimer = setTimeout(() => {
      mediaDragActive.value = false;
      dragOverlayHideTimer = null;
    }, 140);
  }

  function onDrop(event: DragEvent) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    if (!hasFileTransferPayload(event.dataTransfer)) return;
    event.preventDefault();
    const collected = collectDroppedFiles(event);
    if (collected.length === 0) {
      mediaDragActive.value = false;
      return;
    }
    options.setChatError("");
    options.setStatus(`收到拖拽文件 ${collected.length} 个（DOM）。`);
    mediaDragActive.value = false;
    if (dragOverlayHideTimer) {
      clearTimeout(dragOverlayHideTimer);
      dragOverlayHideTimer = null;
    }
    void (async () => {
      for (const item of collected) {
        try {
          const queued = await queueInlineBrowserFile(item.file, item.mime);
          applyQueuedAttachmentResult(queued, apiConfig);
        } catch (error) {
          options.setStatusError("status.pasteImageReadFailed", error);
        }
      }
    })();
  }

  async function onNativeFileDrop(paths: string[]) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    if (!Array.isArray(paths) || paths.length === 0) return;
    options.setChatError("");
    options.setStatus(`收到拖拽文件 ${paths.length} 个（Tauri）。`);

    for (const path of paths) {
      try {
        const queued = await ingestAttachment({ kind: "local-path", path });
        applyQueuedAttachmentResult(queued, apiConfig);
      } catch (error) {
        options.setStatusError("status.pasteImageReadFailed", error);
      }
    }
  }

  function removeClipboardImage(index: number) {
    if (index < 0 || index >= options.clipboardImages.value.length) return;
    options.clipboardImages.value.splice(index, 1);
  }

  async function cleanupChatMedia() {
    await hotkeyRecordTest.cleanupHotkeyRecordTest();
    mediaDragActive.value = false;
    if (dragOverlayHideTimer) {
      clearTimeout(dragOverlayHideTimer);
      dragOverlayHideTimer = null;
    }
  }

  return {
    mediaDragActive,
    hotkeyTestRecording: hotkeyRecordTest.hotkeyTestRecording,
    hotkeyTestRecordingMs: hotkeyRecordTest.hotkeyTestRecordingMs,
    hotkeyTestAudio: hotkeyRecordTest.hotkeyTestAudio,
    microphonePermissionState: hotkeyRecordTest.microphonePermissionState,
    microphonePermissionRequesting: hotkeyRecordTest.microphonePermissionRequesting,
    onPaste,
    onDragOver,
    onDrop,
    onNativeFileDrop,
    queueTextAttachment,
    removeClipboardImage,
    startHotkeyRecordTest: hotkeyRecordTest.startHotkeyRecordTest,
    stopHotkeyRecordTest: hotkeyRecordTest.stopHotkeyRecordTest,
    playHotkeyRecordTest: hotkeyRecordTest.playHotkeyRecordTest,
    requestMicrophonePermission: hotkeyRecordTest.requestMicrophonePermission,
    cleanupChatMedia,
  };
}
