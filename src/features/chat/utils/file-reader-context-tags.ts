import type { IdeContextReferenceItem } from "../../../types/app";

type FileReaderContextCandidates = {
  visible: IdeContextReferenceItem | null;
  selection: IdeContextReferenceItem | null;
};

function normalizedFilePath(path: string | undefined): string {
  return String(path || "").trim().replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

export function clearFileReaderContextCandidates(
  candidates: FileReaderContextCandidates,
  closedPaths?: string[],
): FileReaderContextCandidates {
  if (!closedPaths) {
    return { visible: null, selection: null };
  }
  const normalizedClosedPaths = new Set(closedPaths.map(normalizedFilePath).filter(Boolean));
  if (normalizedClosedPaths.size === 0) return candidates;
  return {
    visible: normalizedClosedPaths.has(normalizedFilePath(candidates.visible?.filePath))
      ? null
      : candidates.visible,
    selection: normalizedClosedPaths.has(normalizedFilePath(candidates.selection?.filePath))
      ? null
      : candidates.selection,
  };
}
