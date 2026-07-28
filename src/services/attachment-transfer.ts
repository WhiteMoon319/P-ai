import { ingestTransportLocalAttachment, uploadTransportAttachment } from "./tauri-api";

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

export type BrowserFileUploader = (file: File) => Promise<AttachmentReceipt>;

export type IngestAttachmentOptions = {
  uploadBrowserFile?: BrowserFileUploader;
  ingestLocalPath?: (source: Extract<AttachmentSource, { kind: "local-path" }>) => Promise<AttachmentReceipt>;
};

export async function uploadBrowserFileThroughTransport(file: File): Promise<AttachmentReceipt> {
  return uploadTransportAttachment<AttachmentReceipt>(file);
}

export async function ingestAttachment(
  source: AttachmentSource,
  options: IngestAttachmentOptions = {},
): Promise<AttachmentReceipt> {
  if (source.kind === "browser-file") {
    const uploader = options.uploadBrowserFile || uploadBrowserFileThroughTransport;
    return await uploader(source.file);
  }
  if (options.ingestLocalPath) {
    return await options.ingestLocalPath(source);
  }
  return await ingestTransportLocalAttachment<AttachmentReceipt>({
    path: source.path,
    fileName: source.fileName,
    mime: source.mime,
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
