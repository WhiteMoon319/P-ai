export function resolveSideChatSelectionAfterClose(
  orderedIds: string[],
  activeId: string,
  closingIds: Iterable<string>,
) {
  const closingSet = new Set(closingIds);
  if (!closingSet.has(activeId)) return activeId;
  const activeIndex = orderedIds.indexOf(activeId);
  return orderedIds.slice(activeIndex + 1).find((item) => !closingSet.has(item))
    || orderedIds.slice(0, Math.max(activeIndex, 0)).reverse().find((item) => !closingSet.has(item))
    || "";
}
