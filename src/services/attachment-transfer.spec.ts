import { describe, expect, it, vi } from "vitest";
import {
  attachmentPreviewBase64,
  base64AttachmentFile,
  ingestAttachment,
  textAttachmentFile,
  type AttachmentReceipt,
} from "./attachment-transfer";

const receipt: AttachmentReceipt = {
  id: "transfer-1",
  fileName: "report.pdf",
  mime: "application/pdf",
  size: 3,
  path: "C:/attachments/report.pdf",
  attachAsMedia: false,
  textNotice: "[附件#1]",
};

describe("attachment-transfer", () => {
  it("routes browser files and local paths through their respective adapters", async () => {
    const browserFile = new File(["abc"], "report.pdf", { type: "application/pdf" });
    const uploadBrowserFile = vi.fn().mockResolvedValue(receipt);
    const ingestLocalPath = vi.fn().mockResolvedValue(receipt);

    await expect(ingestAttachment(
      { kind: "browser-file", file: browserFile },
      { uploadBrowserFile },
    )).resolves.toEqual(receipt);
    await expect(ingestAttachment(
      { kind: "local-path", path: "C:/source/report.pdf" },
      { ingestLocalPath },
    )).resolves.toEqual(receipt);

    expect(uploadBrowserFile).toHaveBeenCalledWith(browserFile);
    expect(ingestLocalPath).toHaveBeenCalledWith({
      kind: "local-path",
      path: "C:/source/report.pdf",
    });
  });

  it("keeps previews bounded to the receipt while preserving text and base64 adapters", () => {
    const previewReceipt = { ...receipt, previewDataUrl: "data:image/png;base64,YWJj" };
    expect(attachmentPreviewBase64(previewReceipt)).toBe("YWJj");

    const textFile = textAttachmentFile("note.md", "hello");
    expect(textFile.name).toBe("note.md");
    expect(textFile.type).toBe("text/markdown");

    const binaryFile = base64AttachmentFile("image.png", "YWJj", "image/png");
    expect(binaryFile.name).toBe("image.png");
    expect(binaryFile.type).toBe("image/png");
    expect(binaryFile.size).toBe(3);
  });
});
