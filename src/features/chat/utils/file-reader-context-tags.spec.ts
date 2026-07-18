import { describe, expect, it } from "vitest";
import type { IdeContextReferenceItem } from "../../../types/app";
import { clearFileReaderContextCandidates } from "./file-reader-context-tags";

const visible = { id: "visible", filePath: "E:/repo/src/one.ts" } as IdeContextReferenceItem;
const selection = { id: "selection", filePath: "E:/repo/src/two.ts" } as IdeContextReferenceItem;

describe("clearFileReaderContextCandidates", () => {
  it("关闭文件时仅清理对应的侧边候选标签", () => {
    expect(clearFileReaderContextCandidates(
      { visible, selection },
      ["e:\\repo\\src\\one.ts"],
    )).toEqual({ visible: null, selection });
  });

  it("关闭阅读器侧边栏时清理全部侧边候选标签", () => {
    expect(clearFileReaderContextCandidates({ visible, selection })).toEqual({
      visible: null,
      selection: null,
    });
  });
});
