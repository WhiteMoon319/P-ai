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

  it("补出的无行号文件标签仍然保留同一路径引用", () => {
    const ranged = {
      id: "ranged",
      filePath: "E:/repo/AGENTS.md",
      relativePath: "AGENTS.md",
      fileName: "AGENTS.md",
      displayLabel: "AGENTS.md:3-32",
      startLine: 3,
      endLine: 32,
    } as IdeContextReferenceItem;

    const groups = mergeComposerIdeContextGroups([
      { workspacePath: "", workspaceName: "", references: [ranged] },
    ], []);

    const fileOnly = groups[0]?.references.find((item) => item.displayLabel === "AGENTS.md");
    expect(fileOnly?.filePath).toBe("E:/repo/AGENTS.md");
    expect(fileOnly?.startLine).toBe(0);
    expect(fileOnly?.endLine).toBe(0);
  });
});
