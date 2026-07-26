import { invoke } from "@tauri-apps/api/core";
import { invokeTauri, isTauriRuntimeAvailable, uploadWebBridgeAttachment } from "./tauri-api";

export const ATTACHMENT_TRANSFER_CHUNK_BYTES = 256 * 1024;
export const WEB_ATTACHMENT_MAX_BYTES = 50 * 1024 * 1024;

export type AttachmentSource =
  | { kind: "local-path"; path: string; fileName?: string; mime?: string }
  | { kind: "browser-file"; file: File };

export type AttachmentReceipt = {
  id: string;
  fileName: string;
  mime: string;
  size: number;
  path: string;
  attachAsMedia: boolean;
  textNotice: string;
  previewDataUrl?: string;
};

type AttachmentTransferBeginOutput = {
  transferId: string;
  nextOffset: number;
  chunkSize?: number;
};

type AttachmentTransferChunkOutput = {
  transferId: string;
  nextOffset: number;
};

export type BrowserFileUploader = (file: File) => Promise<AttachmentReceipt>;

export type IngestAttachmentOptions = {
  uploadBrowserFile?: BrowserFileUploader;
  ingestLocalPath?: (source: Extract<AttachmentSource, { kind: "local-path" }>) => Promise<AttachmentReceipt>;
};

function normalizedFileName(file: File): string {
  return String(file.name || "").trim() || "attachment";
}

async function abortTauriTransfer(transferId: string) {
  try {
    await invokeTauri("attachment_transfer_abort", { input: { transferId } });
  } catch {
    // 完成、断线或后端已清理时无需重复暴露 abort 错误。
  }
}

export async function uploadBrowserFileThroughTauri(file: File): Promise<AttachmentReceipt> {
  if (!isTauriRuntimeAvailable()) {
    return await uploadWebBridgeAttachment<AttachmentReceipt>(file);
  }
  const begin = await invokeTauri<AttachmentTransferBeginOutput>("attachment_transfer_begin", {
    input: {
      fileName: normalizedFileName(file),
      mime: String(file.type || "").trim(),
      size: Number(file.size || 0),
    },
  });
  const transferId = String(begin?.transferId || "").trim();
  if (!transferId) throw new Error("附件传输未返回 transferId");
  const chunkSize = Math.max(
    1,
    Math.min(Number(begin?.chunkSize || ATTACHMENT_TRANSFER_CHUNK_BYTES), ATTACHMENT_TRANSFER_CHUNK_BYTES),
  );
  let offset = Number(begin?.nextOffset || 0);
  try {
    while (offset < file.size) {
      const end = Math.min(file.size, offset + chunkSize);
      const chunk = new Uint8Array(await file.slice(offset, end).arrayBuffer());
      if (chunk.length === 0) throw new Error("附件分块为空");
      let ack: AttachmentTransferChunkOutput | null = null;
      let lastError: unknown = null;
      for (let attempt = 0; attempt < 2; attempt += 1) {
        try {
          ack = await invoke<AttachmentTransferChunkOutput>(
            "attachment_transfer_chunk",
            chunk,
            {
              headers: {
                "x-pai-transfer-id": transferId,
                "x-pai-transfer-offset": String(offset),
              },
            },
          );
          break;
        } catch (error) {
          lastError = error;
          if (attempt === 1) throw error;
        }
      }
      if (!ack) throw lastError instanceof Error ? lastError : new Error("附件分块传输失败");
      const nextOffset = Number(ack?.nextOffset);
      if (!Number.isSafeInteger(nextOffset) || nextOffset <= offset || nextOffset > file.size) {
        throw new Error(`附件分块确认 offset 无效：${String(ack?.nextOffset)}`);
      }
      offset = nextOffset;
    }
    return await invokeTauri<AttachmentReceipt>("attachment_transfer_complete", {
      input: { transferId },
    });
  } catch (error) {
    await abortTauriTransfer(transferId);
    throw error;
  }
}

export async function ingestAttachment(
  source: AttachmentSource,
  options: IngestAttachmentOptions = {},
): Promise<AttachmentReceipt> {
  if (source.kind === "browser-file") {
    const uploader = options.uploadBrowserFile || uploadBrowserFileThroughTauri;
    return await uploader(source.file);
  }
  if (options.ingestLocalPath) {
    return await options.ingestLocalPath(source);
  }
  return await invokeTauri<AttachmentReceipt>("attachment_ingest_local_path", {
    input: {
      path: source.path,
      fileName: source.fileName,
      mime: source.mime,
    },
  });
}

export function attachmentPreviewBase64(receipt: AttachmentReceipt): string {
  const dataUrl = String(receipt.previewDataUrl || "").trim();
  const separator = dataUrl.indexOf(",");
  return separator >= 0 ? dataUrl.slice(separator + 1).trim() : "";
}

export function textAttachmentFile(fileName: string, text: string, mime = "text/markdown"): File {
  return new File([String(text || "")], String(fileName || "").trim() || "attachment.md", {
    type: String(mime || "").trim() || "text/markdown",
  });
}

export function base64AttachmentFile(fileName: string, bytesBase64: string, mime: string): File {
  const decoded = atob(String(bytesBase64 || "").trim());
  const bytes = new Uint8Array(decoded.length);
  for (let index = 0; index < decoded.length; index += 1) {
    bytes[index] = decoded.charCodeAt(index);
  }
  return new File([bytes], String(fileName || "").trim() || "attachment", {
    type: String(mime || "").trim() || "application/octet-stream",
  });
}
