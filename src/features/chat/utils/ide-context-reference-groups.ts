import type { IdeContextReferenceItem, IdeContextWorkspaceGroup } from "../../../types/app";
import { ideContextReferenceDisplayParts } from "./ide-context-reference-display";

function referencePathKey(item: IdeContextReferenceItem): string {
  return String(item.filePath || item.relativePath || item.displayLabel || item.id || "")
    .trim()
    .replace(/\\/g, "/")
    .toLowerCase();
}

function referenceIdentityKey(item: IdeContextReferenceItem): string {
  const path = referencePathKey(item);
  if (!path) return "";
  return [
    path,
    Number(item.startLine || 0),
    Number(item.endLine || 0),
  ].join(":");
}

function sameReferenceRange(left: IdeContextReferenceItem, right: IdeContextReferenceItem): boolean {
  return referencePathKey(left) === referencePathKey(right)
    && Number(left.startLine || 0) === Number(right.startLine || 0)
    && Number(left.endLine || 0) === Number(right.endLine || 0);
}

function referenceDisplayKey(item: IdeContextReferenceItem): string {
  const parts = ideContextReferenceDisplayParts(item);
  return `${parts.fileName}${parts.lineSuffix}`.trim().toLowerCase();
}

export function mergeComposerIdeContextGroups(
  candidateGroups: IdeContextWorkspaceGroup[],
  attachedReferences: IdeContextReferenceItem[],
): IdeContextWorkspaceGroup[] {
  const referencesByIdentity = new Map<string, IdeContextReferenceItem>();
  const referencesByDisplay = new Map<string, IdeContextReferenceItem>();
  const attachedMap = new Map(attachedReferences.map((item) => [item.id, item]));
  const fileOnlyAdded = new Set<string>();
  for (const group of candidateGroups) {
    for (const item of group.references || []) {
      const identity = referenceIdentityKey(item);
      if (!identity) continue;
      const displayKey = referenceDisplayKey(item);
      if (displayKey && referencesByDisplay.has(displayKey)) continue;

      const pathKey = referencePathKey(item);
      const hasLineRange = Number(item.startLine || 0) > 0 || Number(item.endLine || 0) > 0;
      if (hasLineRange && !fileOnlyAdded.has(pathKey)) {
        const fileOnlyItem: IdeContextReferenceItem = {
          ...item,
          id: item.id + "-file-only",
          startLine: 0,
          endLine: 0,
          displayLabel: item.fileName || item.relativePath || item.filePath,
          textBlock: "",
          content: "",
        };
        const fileOnlyIdentity = referenceIdentityKey(fileOnlyItem);
        if (
          fileOnlyIdentity
          && !referencesByIdentity.has(fileOnlyIdentity)
          && !referencesByDisplay.has(referenceDisplayKey(fileOnlyItem))
          && !attachedReferences.some((attached) => sameReferenceRange(attached, fileOnlyItem))
        ) {
          referencesByIdentity.set(fileOnlyIdentity, fileOnlyItem);
          referencesByDisplay.set(referenceDisplayKey(fileOnlyItem), fileOnlyItem);
          fileOnlyAdded.add(pathKey);
        }
      }

      if (attachedReferences.some((attached) => sameReferenceRange(attached, item))) continue;
      referencesByIdentity.set(identity, item);
      if (displayKey) referencesByDisplay.set(displayKey, item);
    }
  }
  for (const item of attachedReferences) {
    const identity = referenceIdentityKey(item);
    if (!identity) continue;
    const displayKey = referenceDisplayKey(item);
    if (displayKey && referencesByDisplay.has(displayKey)) continue;
    referencesByIdentity.set(identity, item);
    if (displayKey) referencesByDisplay.set(displayKey, item);
  }
  const references = Array.from(referencesByIdentity.values()).sort((left, right) => {
    const leftHasLineRange = Number(left.startLine || 0) > 0 || Number(left.endLine || 0) > 0;
    const rightHasLineRange = Number(right.startLine || 0) > 0 || Number(right.endLine || 0) > 0;
    if (leftHasLineRange !== rightHasLineRange) return Number(rightHasLineRange) - Number(leftHasLineRange);
    const leftAttached = attachedMap.has(left.id) ? 1 : 0;
    const rightAttached = attachedMap.has(right.id) ? 1 : 0;
    if (leftAttached !== rightAttached) return rightAttached - leftAttached;
    return String(left.displayLabel || "").localeCompare(String(right.displayLabel || ""));
  });
  return references.length > 0
    ? [{ workspacePath: "", workspaceName: "", references }]
    : [];
}
