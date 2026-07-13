import type { IdeContextReferenceItem } from "../../types/app";
import { CONTEXT_TEXT_BLOCK_CONTENT_LIMIT } from "./constants";
import type { FileTab } from "./types";
import {
  directoryFromPath,
  formatLineSuffix,
  hashText,
  languageIdFromTab,
  normalizePath,
  relativePathFromWorkspace,
  resolveRawSelectedLineRange,
  resolveVisibleLineRange,
  splitContentLines,
  titleFromPath,
} from "./utils";

type TranslateFn = (key: string, params?: Record<string, unknown>) => string;
type LineRange = { startLine?: number; endLine?: number };

export function fileReaderLineReference(path: string, lineRange: LineRange): string {
  return `${normalizePath(path)}${formatLineSuffix(lineRange.startLine, lineRange.endLine)}`;
}

export function buildFileReaderContextMeta(tab: FileTab, initialRootPath: string) {
  const filePath = normalizePath(tab.path);
  const workspacePath = normalizePath(initialRootPath || directoryFromPath(filePath));
  return {
    filePath,
    workspacePath,
    workspaceName: titleFromPath(workspacePath),
    relativePath: relativePathFromWorkspace(filePath, workspacePath),
  };
}

export function resolveFileReaderSelectedLineRange(
  tab: FileTab,
  scroller: HTMLElement,
  selectedText: string,
  range?: Range,
): { startLine: number; endLine: number } {
  if (tab.virtualized) {
    const virtualizedLineRange = range ? resolveVirtualizedSelectedLineRange(range) : null;
    if (virtualizedLineRange) return virtualizedLineRange;
    return resolveVisibleLineRange(scroller, Math.max(1, tab.totalLines));
  }
  if (tab.kind === "markdown" && !tab.rawMode) {
    return resolveVisibleLineRange(scroller, Math.max(1, splitContentLines(tab.content).length));
  }
  return resolveRawSelectedLineRange(tab.content, selectedText)
    || resolveVisibleLineRange(scroller, Math.max(1, splitContentLines(tab.content).length));
}

export function buildFileReaderContextReference(input: {
  tab: FileTab;
  initialRootPath: string;
  source: "selection" | "visible_range";
  lineRange: LineRange;
  content: string;
  displayLabel: string;
  capturedAt: string;
  t: TranslateFn;
}): IdeContextReferenceItem {
  const meta = buildFileReaderContextMeta(input.tab, input.initialRootPath);
  const languageId = languageIdFromTab(input.tab);
  return {
    id: `file-reader-context:${hashText([
      meta.filePath,
      input.source,
      input.lineRange.startLine || "",
      input.lineRange.endLine || "",
    ].join("\n"))}`,
    workspacePath: meta.workspacePath,
    workspaceName: meta.workspaceName,
    filePath: meta.filePath,
    fileName: input.tab.title,
    relativePath: meta.relativePath,
    startLine: input.lineRange.startLine,
    endLine: input.lineRange.endLine,
    displayLabel: input.displayLabel,
    content: input.content,
    languageId,
    source: input.source,
    capturedAt: input.capturedAt,
    textBlock: buildContextTextBlock({
      filePath: meta.filePath,
      lineRange: input.lineRange,
      content: input.content,
      t: input.t,
    }),
  };
}

function resolveVirtualizedSelectedLineRange(range: Range): { startLine: number; endLine: number } | null {
  const startLine = resolveVirtualizedBoundaryLine(range.startContainer, range.startOffset);
  const endLine = resolveVirtualizedBoundaryLine(range.endContainer, range.endOffset);
  if (!startLine || !endLine) return null;
  return { startLine: Math.min(startLine, endLine), endLine: Math.max(startLine, endLine) };
}

function resolveVirtualizedBoundaryLine(container: Node, offset: number): number | null {
  const element = container.nodeType === Node.ELEMENT_NODE ? container as Element : container.parentElement;
  const row = element?.closest<HTMLElement>(".file-reader-code-virtual-row");
  if (!row) return null;
  const blockStartLine = Number(row.dataset.startLine || 0);
  const blockEndLine = Number(row.dataset.endLine || 0);
  if (!Number.isFinite(blockStartLine) || !Number.isFinite(blockEndLine) || blockStartLine <= 0 || blockEndLine < blockStartLine) return null;

  const shikiLine = element?.closest<HTMLElement>(".file-reader-code-virtual-shiki .line");
  if (shikiLine && row.contains(shikiLine)) {
    const lineElements = Array.from(row.querySelectorAll<HTMLElement>(".file-reader-code-virtual-shiki code .line"));
    const index = lineElements.indexOf(shikiLine);
    if (index >= 0) return Math.max(blockStartLine, Math.min(blockEndLine, blockStartLine + index));
  }

  const rawPre = row.querySelector<HTMLElement>(".file-reader-code-virtual-raw");
  if (rawPre && (container === rawPre || rawPre.contains(container))) {
    const lineIndex = lineIndexWithinElement(rawPre, container, offset);
    if (lineIndex != null) return Math.max(blockStartLine, Math.min(blockEndLine, blockStartLine + lineIndex));
  }
  return blockStartLine;
}

function lineIndexWithinElement(root: HTMLElement, container: Node, offset: number): number | null {
  const range = document.createRange();
  try {
    range.selectNodeContents(root);
    range.setEnd(container, offset);
    return Math.max(0, range.toString().split("\n").length - 1);
  } catch {
    return null;
  } finally {
    range.detach();
  }
}

function buildContextTextBlock(input: {
  filePath: string;
  lineRange: LineRange;
  content: string;
  t: TranslateFn;
}): string {
  const location = `${input.filePath}${formatLineSuffix(input.lineRange.startLine, input.lineRange.endLine)}`;
  if (input.content.length > CONTEXT_TEXT_BLOCK_CONTENT_LIMIT) {
    return `${input.t("fileReader.referenceFile", { location })}${input.t("fileReader.referenceTruncated", { count: input.content.length })}`;
  }
  return [input.t("fileReader.referenceFile", { location }), "```text", input.content, "```"].join("\n");
}
