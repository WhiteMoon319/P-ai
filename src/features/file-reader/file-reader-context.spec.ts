import { describe, expect, it } from "vitest";
import type { FileTab } from "./types";
import { buildFileReaderContextReference, fileReaderLineReference } from "./file-reader-context";

const tab: FileTab = {
  path: "E:/repo/src/app.ts",
  title: "app.ts",
  extension: "ts",
  kind: "code",
  content: "const value = 1;",
  rawMode: false,
  forcePlain: false,
  virtualized: false,
  totalLines: 1,
  blockLineCount: 0,
  loaded: true,
  loading: false,
  error: "",
};

const t = (key: string, params?: Record<string, unknown>) => `${key}:${JSON.stringify(params || {})}`;

describe("file-reader-context", () => {
  it("生成稳定的工作区相对引用和行号文本块", () => {
    const reference = buildFileReaderContextReference({
      tab,
      initialRootPath: "E:/repo",
      source: "selection",
      lineRange: { startLine: 3, endLine: 4 },
      content: "const value = 1;",
      displayLabel: "src/app.ts:3-4",
      capturedAt: "2026-07-13T00:00:00Z",
      t,
    });

    expect(reference.workspacePath).toBe("E:/repo");
    expect(reference.relativePath).toBe("src/app.ts");
    expect(reference.startLine).toBe(3);
    expect(reference.textBlock).toContain("E:/repo/src/app.ts:3-4");
    expect(reference.textBlock).toContain("```text");
  });

  it("文件行号引用继续复用标准后缀规则", () => {
    expect(fileReaderLineReference("E:\\repo\\src\\app.ts", { startLine: 8, endLine: 8 }))
      .toBe("E:/repo/src/app.ts:8");
  });
});
