import { describe, expect, it } from "vitest";
import { ideContextReferenceDisplayParts } from "./ide-context-reference-display";

describe("ide-context-reference-display", () => {
  it("完整路径只展示文件名并单独保留行号范围", () => {
    expect(ideContextReferenceDisplayParts({
      fileName: "E:/github/easy_call_ai/pnpm-lock.yaml",
      filePath: "E:/github/easy_call_ai/pnpm-lock.yaml",
      relativePath: "pnpm-lock.yaml",
      displayLabel: "E:/github/easy_call_ai/pnpm-lock.yaml:47-59",
      startLine: 47,
      endLine: 59,
    })).toEqual({
      fileName: "pnpm-lock.yaml",
      lineSuffix: ":47-59",
    });
  });

  it("长文件名与行号保持为两个独立展示字段", () => {
    expect(ideContextReferenceDisplayParts({
      fileName: "这是一个非常非常长并且需要被省略的文件名称.ts",
      filePath: "E:/repo/src/这是一个非常非常长并且需要被省略的文件名称.ts",
      relativePath: "src/这是一个非常非常长并且需要被省略的文件名称.ts",
      displayLabel: "src/这是一个非常非常长并且需要被省略的文件名称.ts:123456",
      startLine: 123456,
      endLine: 123456,
    })).toEqual({
      fileName: "这是一个非常非常长并且需要被省略的文件名称.ts",
      lineSuffix: ":123456",
    });
  });
});
