import { describe, expect, it } from "vitest";
import { buildChatIngressParts } from "./use-chat-flow-send-controller";

describe("buildChatIngressParts", () => {
  it("builds ordered canonical parts without legacy attachment mirrors", () => {
    const parts = buildChatIngressParts(
      "请处理附件",
      [
        {
          mime: "image/png",
          bytesBase64: "ignored-because-path-is-authoritative",
          savedPath: "C:\\workspace\\downloads\\source.png",
        },
        {
          mime: "image/jpeg",
          bytesBase64: "raw-image",
        },
      ],
      [
        {
          fileName: "source.png",
          path: "C:/workspace/downloads/source.png",
          mime: "image/png",
        },
        {
          fileName: "report.pdf",
          path: "C:/workspace/downloads/report.pdf",
          mime: "application/pdf",
        },
      ],
    );

    expect(parts).toEqual([
      { type: "text", text: "请处理附件" },
      {
        type: "attachment",
        path: "C:/workspace/downloads/source.png",
        mime: "image/png",
        name: "source.png",
      },
      {
        type: "attachment",
        bytesBase64: "raw-image",
        mime: "image/jpeg",
        name: "image",
      },
      {
        type: "attachment",
        path: "C:/workspace/downloads/report.pdf",
        mime: "application/pdf",
        name: "report.pdf",
      },
    ]);
    expect(JSON.stringify(parts)).not.toContain("relativePath");
    expect(JSON.stringify(parts)).not.toContain("attachments");
  });
});
