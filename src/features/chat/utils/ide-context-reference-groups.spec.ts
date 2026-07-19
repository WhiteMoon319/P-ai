import { describe, expect, it } from "vitest";
import type { IdeContextReferenceItem } from "../../../types/app";
import { mergeComposerIdeContextGroups } from "./ide-context-reference-groups";

describe("ide-context-reference-groups", () => {
  it("候选标签全部隐藏时仍保留已确认添加的标签", () => {
    const attached = {
      id: "attached-file",
      filePath: "E:/repo/src/app.ts",
      relativePath: "src/app.ts",
      displayLabel: "src/app.ts:8",
      startLine: 8,
      endLine: 8,
    } as IdeContextReferenceItem;

    const groups = mergeComposerIdeContextGroups([], [attached]);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.references.map((item) => item.id)).toEqual(["attached-file"]);
  });

  it("按最终展示文本去重，避免同名文件标签重复出现", () => {
    const first = {
      id: "first",
      filePath: "E:/repo/a/AGENTS.md",
      relativePath: "a/AGENTS.md",
      displayLabel: "AGENTS.md",
      fileName: "AGENTS.md",
      startLine: 0,
      endLine: 0,
    } as IdeContextReferenceItem;
    const second = {
      id: "second",
      filePath: "E:/repo/b/AGENTS.md",
      relativePath: "b/AGENTS.md",
      displayLabel: "AGENTS.md",
      fileName: "AGENTS.md",
      startLine: 0,
      endLine: 0,
    } as IdeContextReferenceItem;

    const groups = mergeComposerIdeContextGroups([
      { workspacePath: "", workspaceName: "", references: [first, second] },
    ], []);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.references.map((item) => item.displayLabel)).toEqual([
      "AGENTS.md",
    ]);
  });
});
