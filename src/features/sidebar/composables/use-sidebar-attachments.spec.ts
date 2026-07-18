import { afterEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useSidebarAttachments } from "./use-sidebar-attachments";

class FakeFileReader {
  result: string | ArrayBuffer | null = null;
  error: DOMException | null = null;
  onload: ((event: ProgressEvent<FileReader>) => void) | null = null;
  onerror: ((event: ProgressEvent<FileReader>) => void) | null = null;

  readAsDataURL(_blob: Blob) {
    this.result = "data:image/png;base64,YWJj";
    queueMicrotask(() => this.onload?.({} as ProgressEvent<FileReader>));
  }
}

function file(name: string): File {
  return { name, type: "image/png" } as File;
}

function setup(queueAttachment: any) {
  const errorText = ref("");
  const attachments = useSidebarAttachments({
    view: ref("chat"),
    busy: ref(false),
    compacting: ref(false),
    errorText,
    t: (key) => key,
    queueAttachment,
  });
  return { attachments, errorText };
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("useSidebarAttachments", () => {
  it("continues with later files when one attachment fails", async () => {
    vi.stubGlobal("FileReader", FakeFileReader);
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const queueAttachment = vi.fn()
      .mockRejectedValueOnce(new Error("first failed"))
      .mockResolvedValueOnce({
        mime: "image/png",
        fileName: "second.png",
        savedPath: "C:/attachments/second.png",
        attachAsMedia: true,
        bytesBase64: "YWJj",
      });
    const { attachments, errorText } = setup(queueAttachment);

    await attachments.appendAttachmentFiles([file("first.png"), file("second.png")]);

    expect(queueAttachment).toHaveBeenCalledTimes(2);
    expect(attachments.queuedAttachmentEntries.value).toHaveLength(1);
    expect(attachments.queuedAttachmentEntries.value[0]?.path).toBe("C:/attachments/second.png");
    expect(errorText.value).toContain("first failed");
  });

  it("never fabricates a relative path when queue result has no saved path", async () => {
    vi.stubGlobal("FileReader", FakeFileReader);
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const queueAttachment = vi.fn().mockResolvedValue({
      mime: "application/pdf",
      fileName: "report.pdf",
      savedPath: "",
      attachAsMedia: false,
      bytesBase64: null,
    });
    const { attachments, errorText } = setup(queueAttachment);

    await attachments.appendAttachmentFiles([
      { name: "report.pdf", type: "application/pdf" } as File,
    ]);

    expect(attachments.queuedAttachmentEntries.value).toEqual([]);
    expect(attachments.buildQueuedAttachmentPayload()).toEqual([]);
    expect(errorText.value).toContain("绝对路径");
  });
});
