import { afterEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useSidebarAttachments } from "./use-sidebar-attachments";

function file(name: string): File {
  return { name, type: "image/png" } as File;
}

function setup(uploadAttachment: any) {
  const errorText = ref("");
  const attachments = useSidebarAttachments({
    view: ref("chat"),
    busy: ref(false),
    compacting: ref(false),
    errorText,
    t: (key) => key,
    uploadAttachment,
  });
  return { attachments, errorText };
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("useSidebarAttachments", () => {
  it("continues with later files when one attachment fails", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const uploadAttachment = vi.fn()
      .mockRejectedValueOnce(new Error("first failed"))
      .mockResolvedValueOnce({
        id: "transfer-2",
        mime: "image/png",
        fileName: "second.png",
        path: "C:/attachments/second.png",
        size: 3,
        attachAsMedia: true,
        textNotice: "",
        previewDataUrl: "data:image/png;base64,YWJj",
      });
    const { attachments, errorText } = setup(uploadAttachment);

    await attachments.appendAttachmentFiles([file("first.png"), file("second.png")]);

    expect(uploadAttachment).toHaveBeenCalledTimes(2);
    expect(uploadAttachment.mock.calls[1]?.[0]).toMatchObject({ name: "second.png", type: "image/png" });
    expect(attachments.queuedAttachmentEntries.value).toHaveLength(1);
    expect(attachments.queuedAttachmentEntries.value[0]?.path).toBe("C:/attachments/second.png");
    expect(errorText.value).toContain("first failed");
  });

  it("never fabricates a relative path when queue result has no saved path", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const uploadAttachment = vi.fn().mockResolvedValue({
      id: "transfer-pdf",
      mime: "application/pdf",
      fileName: "report.pdf",
      path: "",
      size: 3,
      attachAsMedia: false,
      textNotice: "",
    });
    const { attachments, errorText } = setup(uploadAttachment);

    await attachments.appendAttachmentFiles([
      { name: "report.pdf", type: "application/pdf" } as File,
    ]);

    expect(attachments.queuedAttachmentEntries.value).toEqual([]);
    expect(attachments.buildQueuedAttachmentPayload()).toEqual([]);
    expect(errorText.value).toContain("绝对路径");
  });
});
