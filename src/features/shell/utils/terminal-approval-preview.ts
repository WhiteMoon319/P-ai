export type TerminalApprovalPatchKind = "update" | "add" | "delete" | "other";

export type TerminalApprovalImpactItem = {
  path: string;
  adds: number;
  removes: number;
  kind: TerminalApprovalPatchKind;
};

export function splitTerminalApprovalPatches(raw: string): string[][] {
  const normalized = String(raw || "").replace(/\r/g, "");
  const lines = normalized.split("\n");
  if (!normalized.trim()) return [[]];

  const patches: string[][] = [];
  let currentPatch: string[] = [];
  let inPatchBlock = false;

  for (const rawLine of lines) {
    const line = String(rawLine || "");
    const trimmedLine = line.trim();
    if (trimmedLine.startsWith("*** Begin Patch")) {
      if (currentPatch.length > 0) {
        patches.push(currentPatch);
      }
      currentPatch = [line];
      inPatchBlock = true;
      continue;
    }

    if (inPatchBlock && trimmedLine.startsWith("*** End Patch")) {
      currentPatch.push(line);
      patches.push(currentPatch);
      currentPatch = [];
      inPatchBlock = false;
      continue;
    }

    currentPatch.push(line);
  }

  if (currentPatch.length > 0) {
    patches.push(currentPatch);
  }

  return patches.length > 0 ? patches : [lines];
}

export function getTerminalApprovalPatchPath(lines: string[]): string {
  for (const rawLine of lines) {
    const line = String(rawLine || "").trim();
    if (line.startsWith("*** Update File:")) {
      return line.replace("*** Update File:", "").trim();
    }
    if (line.startsWith("*** Add File:")) {
      return line.replace("*** Add File:", "").trim();
    }
    if (line.startsWith("*** Delete File:")) {
      return line.replace("*** Delete File:", "").trim();
    }
  }
  return "";
}

export function getTerminalApprovalPatchKind(lines: string[]): TerminalApprovalPatchKind {
  for (const rawLine of lines) {
    const line = String(rawLine || "").trim();
    if (line.startsWith("*** Update File:")) return "update";
    if (line.startsWith("*** Add File:")) return "add";
    if (line.startsWith("*** Delete File:")) return "delete";
  }
  return "other";
}

export function countTerminalApprovalPatchDelta(lines: string[]) {
  let adds = 0;
  let removes = 0;
  for (const rawLine of lines) {
    const line = String(rawLine || "");
    if (line.startsWith("+") && !line.startsWith("+++")) {
      adds += 1;
      continue;
    }
    if (line.startsWith("-") && !line.startsWith("---")) {
      removes += 1;
    }
  }
  return { adds, removes };
}

export function countTerminalApprovalDiffLines(lines: string[]): number {
  const delta = countTerminalApprovalPatchDelta(lines);
  return delta.adds + delta.removes;
}

export function terminalApprovalImpactFromPatchText(raw: string): TerminalApprovalImpactItem[] {
  return splitTerminalApprovalPatches(raw)
    .map((lines) => {
      const path = getTerminalApprovalPatchPath(lines);
      if (!path) return null;
      return {
        path,
        kind: getTerminalApprovalPatchKind(lines),
        ...countTerminalApprovalPatchDelta(lines),
      };
    })
    .filter((item): item is TerminalApprovalImpactItem => !!item);
}
