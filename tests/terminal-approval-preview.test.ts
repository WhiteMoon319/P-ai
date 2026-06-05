import { describe, expect, it } from "vitest";
import {
  countTerminalApprovalDiffLines,
  splitTerminalApprovalPatches,
  terminalApprovalImpactFromPatchText,
} from "../src/features/shell/utils/terminal-approval-preview";

describe("terminal approval preview", () => {
  it("keeps apply_patch content visible for approval dialog", () => {
    const preview = [
      "*** Begin Patch",
      "*** Update File: E:/github/easy_call_ai/src/main.rs",
      "-let value = ;",
      "+let value = 1;",
      "*** End Patch",
    ].join("\n");

    const blocks = splitTerminalApprovalPatches(preview);
    const impact = terminalApprovalImpactFromPatchText(preview);

    expect(blocks).toHaveLength(1);
    expect(countTerminalApprovalDiffLines(blocks[0])).toBe(2);
    expect(impact).toEqual([
      {
        path: "E:/github/easy_call_ai/src/main.rs",
        kind: "update",
        adds: 1,
        removes: 1,
      },
    ]);
  });

  it("does not treat file-only apply_patch summary as visible diff content", () => {
    const oldSummary = "  update E:/github/easy_call_ai/src/main.rs";
    const blocks = splitTerminalApprovalPatches(oldSummary);

    expect(countTerminalApprovalDiffLines(blocks[0])).toBe(0);
    expect(terminalApprovalImpactFromPatchText(oldSummary)).toEqual([]);
  });
});
