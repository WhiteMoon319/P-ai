import { describe, expect, it } from "vitest";
import {
  displayFileName,
  displayLabelFromExtraTextReference,
  extraTextReferenceDisplayParts,
  fileNameFromPath,
} from "./chat-attachment-display";

describe("chat attachment display", () => {
  it("只保留 Windows 或 Unix 路径中的文件名", () => {
    expect(fileNameFromPath("E:\\repo\\src\\ChatMessageItem.vue:260-348")).toBe("ChatMessageItem.vue");
    expect(fileNameFromPath("/repo/src/design-notes.md")).toBe("design-notes.md");
  });

  it("优先使用文件名并兼容缺少文件名的旧附件", () => {
    expect(displayFileName("src\\design-notes.md", "E:\\repo\\design-notes.md")).toBe("design-notes.md");
    expect(displayFileName("", "/tmp/screenshot.png")).toBe("screenshot.png");
  });

  it("从 IDE 文本块或旧翻译文本中提取文件名", () => {
    expect(displayLabelFromExtraTextReference([
      "[IDE 上下文引用]",
      "文件: E:/repo/src/ChatMessageItem.vue",
      "行号: 260-348",
      "内容:",
    ].join("\n"))).toBe("ChatMessageItem.vue:260-348");
    expect(displayLabelFromExtraTextReference("用户引用了文件片段：src/components/DemoTab.vue（第 20 行）")).toBe("DemoTab.vue");
  });

  it("从 IDE 文本块中保留行号后缀", () => {
    expect(extraTextReferenceDisplayParts([
      "[IDE 上下文引用]",
      "文件: E:/repo/src/ChatMessageItem.vue",
      "行号: 260-348",
      "内容:",
    ].join("\n"))).toEqual({
      fileName: "ChatMessageItem.vue",
      lineSuffix: ":260-348",
    });
  });
});
