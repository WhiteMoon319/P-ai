import { describe, expect, it } from "vitest";
import type { FileTab } from "./types";
import {
  buildFileReaderContextReference,
  buildFileReaderSelectionContextReference,
  fileReaderLineReference,
  resolveFileReaderSelectionActionPosition,
  resolveFileReaderSelectedLineRange,
} from "./file-reader-context";

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

  it("超过 2000 字符时只保留路径和行号", () => {
    const reference = buildFileReaderContextReference({
      tab,
      initialRootPath: "E:/repo",
      source: "selection",
      lineRange: { startLine: 3, endLine: 4 },
      content: "a".repeat(2001),
      displayLabel: "src/app.ts:3-4",
      capturedAt: "2026-07-19T00:00:00Z",
      t,
    });

    expect(reference.textBlock).toBe('fileReader.referenceFile:{"location":"E:/repo/src/app.ts:3-4"}');
  });

  it("文件行号引用继续复用标准后缀规则", () => {
    expect(fileReaderLineReference("E:\\repo\\src\\app.ts", { startLine: 8, endLine: 8 }))
      .toBe("E:/repo/src/app.ts:8");
  });

  it("添加到聊天时生成选区行号文件标签", () => {
    const reference = buildFileReaderSelectionContextReference({
      tab,
      initialRootPath: "E:/repo",
      lineRange: { startLine: 12, endLine: 15 },
      selectedText: "const value = 1;",
      capturedAt: "2026-07-17T00:00:00Z",
      t,
    });

    expect(reference.displayLabel).toBe("src/app.ts:12-15");
    expect(reference.startLine).toBe(12);
    expect(reference.endLine).toBe(15);
    expect(reference.source).toBe("selection");
  });

  it("渲染态 Markdown 优先按源文定位选区行号", () => {
    const markdownTab: FileTab = {
      ...tab,
      path: "E:/repo/README.md",
      title: "README.md",
      extension: "md",
      kind: "markdown",
      content: "# 标题\n第一段\n需要添加到聊天的内容\n最后一段",
      rawMode: false,
      totalLines: 4,
    };
    const scroller = {
      scrollHeight: 1000,
      clientHeight: 100,
      scrollTop: 0,
    } as HTMLElement;

    expect(resolveFileReaderSelectedLineRange(
      markdownTab,
      scroller,
      "需要添加到聊天的内容",
    )).toEqual({ startLine: 3, endLine: 3 });
  });

  it("选区操作浮层始终限制在正文区域内", () => {
    expect(resolveFileReaderSelectionActionPosition({
      anchorX: 790,
      anchorY: 180,
      containerRect: { left: 100, right: 700, top: 40, bottom: 600 },
    })).toEqual({
      x: 460,
      y: 188,
    });
  });
});
