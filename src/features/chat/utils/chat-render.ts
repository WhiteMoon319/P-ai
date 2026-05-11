import type { ChatMessageBlock } from "../../../types/app";

// ==================== 类型 ====================

export type ChatRenderItem =
  | { kind: "compaction"; id: string; renderId: string; block: ChatMessageBlock; blockIndex: number }
  | { kind: "plan_started"; id: string; renderId: string; block: ChatMessageBlock; blockIndex: number }
  | { kind: "message"; id: string; renderId: string; block: ChatMessageBlock; blockIndex: number; compactWithPrevious: boolean }
  | { kind: "group"; id: string; groupId: string; items: Array<{ renderId: string; block: ChatMessageBlock; blockIndex: number; compactWithPrevious: boolean }> };

// ==================== 常量 ====================

export const FILE_READER_EXTENSIONS = new Set([
  "md", "markdown", "mdx",
  "ts", "tsx", "c", "cc", "cpp", "cxx", "h", "hpp",
  "cs", "java", "kt", "kts", "go",
  "js", "jsx", "vue", "rs", "py", "rb", "php",
  "swift", "scala", "dart", "lua", "r", "m", "mm",
  "pl", "pm", "json", "jsonc", "json5", "toml",
  "yaml", "yml", "css", "scss", "sass", "less",
  "html", "htm", "xml", "svg", "sql",
  "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
  "dockerfile", "ini", "env",
  "gitignore", "gitattributes", "editorconfig",
  "lock", "csv", "tsv", "txt", "log",
]);

// ==================== 纯函数 ====================

export function fileExtensionFromPath(path: string): string {
  const normalizedPath = String(path || "").trim().replace(/\\/g, "/");
  const fileName = normalizedPath.split("/").filter(Boolean).pop() || "";
  const normalizedFileName = fileName.startsWith(".") ? fileName.slice(1) : fileName;
  const lowerFileName = normalizedFileName.toLowerCase();
  if (FILE_READER_EXTENSIONS.has(lowerFileName)) return lowerFileName;
  const dotIndex = normalizedFileName.lastIndexOf(".");
  if (dotIndex <= 0 || dotIndex === normalizedFileName.length - 1) return "";
  return normalizedFileName.slice(dotIndex + 1).toLowerCase();
}

export function canOpenInFileReader(path: string): boolean {
  return FILE_READER_EXTENSIONS.has(fileExtensionFromPath(path));
}

function countFenceMatches(text: string, pattern: RegExp): number {
  if (!text) return 0;
  return Array.from(text.matchAll(pattern)).length;
}

export function estimateMessageBlockHeight(block: ChatMessageBlock, isOwn: boolean): number {
  let estimate = isOwn ? 78 : 108;
  const text = String(block.text || "");
  const inlineReasoning = String(block.reasoningInline || "");
  const standardReasoning = String(block.reasoningStandard || "");
  const combinedTextLength = text.length + inlineReasoning.length + standardReasoning.length;
  estimate += Math.min(920, Math.ceil(combinedTextLength / 28) * 9);

  const codeFenceCount = countFenceMatches(text, /```[\w-]*\s*[\r\n]/g);
  const mermaidFenceCount = countFenceMatches(text, /```(?:\s*)mermaid\b/gi);
  estimate += codeFenceCount * 180;
  estimate += mermaidFenceCount * 120;

  if (block.planCard) estimate += 84;
  if (block.taskTrigger) estimate += 120;
  if (standardReasoning.trim()) estimate += Math.min(240, Math.ceil(standardReasoning.length / 36) * 12);
  if (inlineReasoning.trim()) estimate += Math.min(180, Math.ceil(inlineReasoning.length / 36) * 10);
  if (block.toolCalls.length > 0) estimate += block.toolCalls.length * 56 + 36;
  if (Array.isArray(block.memeSegments) && block.memeSegments.length > 0) {
    estimate += block.memeSegments.length * 42;
  }
  estimate += block.images.length * 120;
  estimate += block.audios.length * 42;
  estimate += block.attachmentFiles.length * 34;
  return Math.max(64, estimate);
}

export function isCompactionBlock(block: ChatMessageBlock): boolean {
  if (block.remoteImOrigin) return false;
  const meta = (block.providerMeta || {}) as Record<string, unknown>;
  const messageMeta = ((meta.message_meta || meta.messageMeta || {}) as Record<string, unknown>);
  const kind = String(messageMeta.kind || "").trim();
  return kind === "context_compaction" || kind === "summary_context_seed";
}

export function isRightAlignedMessage(block: ChatMessageBlock): boolean {
  if (block.remoteImOrigin) return false;
  if (block.role === "user") return true;
  const id = String(block.speakerAgentId || "").trim();
  return id === "user-persona";
}

export function isCompactUserContinuation(block: ChatMessageBlock, previousBlock: ChatMessageBlock | null): boolean {
  if (!previousBlock) return false;
  if (!isRightAlignedMessage(block) || !isRightAlignedMessage(previousBlock)) return false;
  if (block.dividerKind || previousBlock.dividerKind) return false;
  if (block.isExtraTextBlock || previousBlock.isExtraTextBlock) return false;
  return true;
}

export function blockSizeDependencies(block: ChatMessageBlock): unknown[] {
  return [
    String(block.id || ""),
    String(block.sourceMessageId || ""),
    String(block.text || ""),
    String(block.reasoningInline || ""),
    String(block.reasoningStandard || ""),
    block.images.length,
    block.audios.length,
    block.attachmentFiles.length,
    block.toolCalls.length,
    Array.isArray(block.memeSegments) ? block.memeSegments.length : 0,
    block.planCard?.action || "",
    block.planCard?.path || "",
    String(block.taskTrigger ? JSON.stringify(block.taskTrigger) : ""),
  ];
}
