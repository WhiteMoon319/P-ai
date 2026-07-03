import { CODE_LANGUAGE_BY_EXTENSION, SHIKI_LANGUAGE_KEYS } from "./constants";
import type { FileReaderDirectoryEntry, FileTab, VirtualCodeBlock } from "./types";

export function normalizePath(path: string) {
  return String(path || "")
    .trim()
    .replace(/^\\\\\?\\/, "")
    .replace(/^\/\/\?\//, "")
    .replace(/^\/\?\//, "")
    .replace(/^\?\//, "")
    .replace(/^\?\\/, "")
    .replace(/\\/g, "/")
    .replace(/^\/([A-Za-z]:)/, "$1");
}

export function sameNormalizedPath(left: string, right: string) {
  return normalizePath(left).toLowerCase() === normalizePath(right).toLowerCase();
}

export function splitContentLines(value: string) {
  const normalized = String(value || "").replace(/\r\n/g, "\n");
  return normalized.length > 0 ? normalized.split("\n") : [];
}

export function buildFileBlockKey(path: string, startLine: number, endLine: number) {
  return `${normalizePath(path)}::${startLine}-${endLine}`;
}

export function blockLineNumbers(block: VirtualCodeBlock) {
  return Array.from({ length: block.lineCount }, (_, index) => block.startLine + index);
}

export function normalizeSelectedText(value: string) {
  return String(value || "")
    .replace(/\r\n/g, "\n")
    .replace(/\u00a0/g, " ")
    .trim()
    .slice(0, 20_000);
}

export function resolveVisibleLineRange(scroller: HTMLElement, totalLines: number): { startLine: number; endLine: number } {
  const scrollableHeight = Math.max(1, scroller.scrollHeight - scroller.clientHeight);
  const startRatio = Math.max(0, Math.min(1, scroller.scrollTop / scrollableHeight));
  const visibleRatio = Math.max(0.05, Math.min(1, scroller.clientHeight / Math.max(1, scroller.scrollHeight)));
  const startLine = Math.max(1, Math.min(totalLines, Math.floor(startRatio * totalLines) + 1));
  const visibleLineCount = Math.max(12, Math.ceil(totalLines * visibleRatio));
  const endLine = Math.max(startLine, Math.min(totalLines, startLine + visibleLineCount - 1));
  return { startLine, endLine };
}

export function resolveRawSelectedLineRange(source: string, selectedText: string): { startLine: number; endLine: number } | null {
  const normalizedSource = String(source || "").replace(/\r\n/g, "\n");
  const normalizedSelection = selectedText.replace(/\r\n/g, "\n");
  const index = normalizedSource.indexOf(normalizedSelection);
  if (index < 0) return null;
  if (normalizedSource.indexOf(normalizedSelection, index + Math.max(1, normalizedSelection.length)) >= 0) return null;
  const before = normalizedSource.slice(0, index);
  const startLine = before.split("\n").length;
  const selectedLineCount = Math.max(1, normalizedSelection.split("\n").length);
  return { startLine, endLine: startLine + selectedLineCount - 1 };
}

export function relativePathFromWorkspace(filePath: string, workspacePath: string) {
  const normalizedFilePath = normalizePath(filePath);
  const normalizedWorkspacePath = normalizePath(workspacePath).replace(/\/+$/, "");
  if (!normalizedWorkspacePath) return normalizedFilePath;
  const lowerFilePath = normalizedFilePath.toLowerCase();
  const lowerWorkspacePath = normalizedWorkspacePath.toLowerCase();
  if (lowerFilePath === lowerWorkspacePath) return titleFromPath(normalizedFilePath);
  const prefix = `${lowerWorkspacePath}/`;
  if (lowerFilePath.startsWith(prefix)) {
    return normalizedFilePath.slice(normalizedWorkspacePath.length + 1);
  }
  return normalizedFilePath;
}

export function languageIdFromTab(tab: FileTab) {
  return CODE_LANGUAGE_BY_EXTENSION[tab.extension] || tab.extension || tab.kind || "text";
}

export function formatLineSuffix(startLine?: number, endLine?: number) {
  if (!startLine) return "";
  if (endLine && endLine > startLine) return `:${startLine}-${endLine}`;
  return `:${startLine}`;
}

export function hashText(value: string) {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

export function fileKindFromPath(path: string) {
  const extension = extensionFromPath(path);
  return ["md", "markdown", "mdx"].includes(extension) ? "markdown" : "code";
}

export function extensionFromPath(path: string) {
  const fileName = titleFromPath(path);
  const lowerFileName = fileName.toLowerCase();
  if (CODE_LANGUAGE_BY_EXTENSION[lowerFileName]) return lowerFileName;
  if (SHIKI_LANGUAGE_KEYS.has(lowerFileName)) return lowerFileName;
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex <= 0 || dotIndex === fileName.length - 1) return "";
  const extension = fileName.slice(dotIndex + 1).toLowerCase();
  return CODE_LANGUAGE_BY_EXTENSION[extension] || SHIKI_LANGUAGE_KEYS.has(extension) ? extension : "";
}

export function escapeHtml(value: string) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export function stripMarkdownHtmlComments(value: string) {
  return String(value || "").replace(/<!--[\s\S]*?-->/g, "");
}

export function resolveShikiLanguage(extension: string) {
  const key = String(extension || "").trim().toLowerCase();
  const mapped = CODE_LANGUAGE_BY_EXTENSION[key] || key;
  return SHIKI_LANGUAGE_KEYS.has(mapped) ? mapped : "text";
}

export function titleFromPath(path: string) {
  const normalized = normalizePath(path);
  return normalized.split("/").filter(Boolean).pop() || normalized || "未命名文件";
}

export function directoryFromPath(path: string) {
  const normalized = normalizePath(path).replace(/\/+$/, "");
  const slashIndex = normalized.lastIndexOf("/");
  if (slashIndex < 0) return "";
  if (slashIndex === 0) return "/";
  if (slashIndex === 2 && normalized[1] === ":") return normalized.slice(0, 3);
  return normalized.slice(0, slashIndex);
}

export function normalizeDirectoryEntries(entries: FileReaderDirectoryEntry[]) {
  return entries.map((entry) => ({
    ...entry,
    path: normalizePath(entry.path),
    name: String(entry.name || titleFromPath(entry.path)),
    isDirectory: !!entry.isDirectory,
  }));
}
