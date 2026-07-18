import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../types/app";
import {
  extractMessageAttachmentFiles,
  extractMessageAudios,
  extractMessageImages,
} from "./chat-message";

describe("canonical chat attachments", () => {
  it("projects attachment paths by MIME while keeping legacy inputs readable", () => {
    const message: ChatMessage = {
      id: "message-1",
      role: "user",
      parts: [
        { type: "attachment", path: "C:/downloads/a.png", mime: "image/png", name: "a.png" },
        { type: "attachment", path: "C:/downloads/voice.mp3", mime: "audio/mpeg", name: "voice.mp3" },
        { type: "attachment", path: "C:/downloads/report.pdf", mime: "application/pdf", name: "report.pdf" },
        { type: "image", mime: "image/webp", bytesBase64: "@download:legacy/a.webp" },
      ],
      providerMeta: {
        attachments: [
          { fileName: "legacy.txt", relativePath: "downloads/legacy.txt", mime: "text/plain" },
        ],
      },
    };

    expect(extractMessageImages(message)).toEqual([
      { mime: "image/png", mediaRef: "C:/downloads/a.png", name: "a.png" },
      { mime: "image/webp", bytesBase64: undefined, mediaRef: "@download:legacy/a.webp" },
    ]);
    expect(extractMessageAudios(message)).toEqual([
      { mime: "audio/mpeg", mediaRef: "C:/downloads/voice.mp3" },
    ]);
    expect(extractMessageAttachmentFiles(message)).toEqual([
      { fileName: "report.pdf", path: "C:/downloads/report.pdf", mime: "application/pdf" },
      { fileName: "legacy.txt", path: "downloads/legacy.txt", mime: "text/plain" },
    ]);
  });
});
