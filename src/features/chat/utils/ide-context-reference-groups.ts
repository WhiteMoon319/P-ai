import type { IdeContextReferenceItem, IdeContextWorkspaceGroup } from "../../../types/app";

function referenceTextBlock(item: IdeContextReferenceItem): string {
  const filePath = String(item.filePath || item.relativePath || item.fileName || item.displayLabel || "").trim();
  if (!filePath) return "";
  const startLine = Number(item.startLine || 0);
  const endLine = Number(item.endLine || 0);
  const lineSuffix = startLine > 0 && endLine > startLine
    ? `:${startLine}-${endLine}`
    : startLine > 0
      ? `:${startLine}`
      : "";
  return `用户引用了文件片段：${filePath}${lineSuffix}`;
}

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

export function mergeComposerIdeContextGroups(
  candidateGroups: IdeContextWorkspaceGroup[],
  attachedReferences: IdeContextReferenceItem[],
): IdeContextWorkspaceGroup[] {
  const referencesByIdentity = new Map<string, IdeContextReferenceItem>();
  const attachedMap = new Map(attachedReferences.map((item) => [item.id, item]));
  const fileOnlyAdded = new Set<string>();
  for (const group of candidateGroups) {
    for (const item of group.references || []) {
      const identity = referenceIdentityKey(item);
      if (!identity) continue;

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
        fileOnlyItem.textBlock = referenceTextBlock(fileOnlyItem);
        const fileOnlyIdentity = referenceIdentityKey(fileOnlyItem);
        if (
          fileOnlyIdentity
          && !referencesByIdentity.has(fileOnlyIdentity)
          && !attachedReferences.some((attached) => sameReferenceRange(attached, fileOnlyItem))
        ) {
          referencesByIdentity.set(fileOnlyIdentity, fileOnlyItem);
          fileOnlyAdded.add(pathKey);
        }
      }

      if (attachedReferences.some((attached) => sameReferenceRange(attached, item))) continue;
      referencesByIdentity.set(identity, item);
    }
  }
  for (const item of attachedReferences) {
    const identity = referenceIdentityKey(item);
    if (!identity) continue;
    referencesByIdentity.set(identity, item);
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
