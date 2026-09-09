import type { ChatActivityItem } from "../../../types/app";
import { isAbsoluteLocalPath, normalizeLocalLinkHref } from "./local-link";

export type ToolcallPreviewEntry = {
  title: string;
  body: string;
  /** 可点击打开的本地文件路径（绝对或工作区相对） */
  filePath?: string;
  /** 展示用路径文本；默认等于 filePath */
  fileLabel?: string;
};

const FILE_PATH_KEYS = [
  "absolute_path",
  "absolutePath",
  "path",
  "file",
  "target",
  "source",
  "destination",
  "from",
  "to",
] as const;

function looksLikeLocalPath(value: string): boolean {
  const text = String(value || "").trim();
  if (!text) return false;
  if (isAbsoluteLocalPath(text)) return true;
  if (text.startsWith("./") || text.startsWith("../")) return true;
  // 仓库内相对路径：含路径分隔且不像纯命令
  if ((text.includes("/") || text.includes("\\")) && !/\s/.test(text) && !text.startsWith("-")) {
    return true;
  }
  return false;
}

function pickPathFromRecord(data: Record<string, unknown>): string {
  for (const key of FILE_PATH_KEYS) {
    const raw = data[key];
    if (typeof raw !== "string") continue;
    const value = raw.trim();
    if (!value || !looksLikeLocalPath(value)) continue;
    return normalizeLocalLinkHref(value) || value;
  }
  return "";
}

function readToolOffset(data: Record<string, unknown>): number | undefined {
  const raw = data.offset ?? data.start;
  const parsed = typeof raw === "number" ? raw : Number.parseInt(String(raw ?? ""), 10);
  if (!Number.isFinite(parsed) || parsed < 1) return undefined;
  return Math.floor(parsed);
}

/**
 * 从工具参数中提取首个可打开的文件路径。
 * 仅用于预览展示/点击打开，不改消息语义。
 */
export function extractToolcallFilePath(toolName: string, argsText: string): string {
  const name = String(toolName || "").trim().toLowerCase();
  const text = String(argsText || "").trim();
  if (!text) return "";

  // 命令类工具不把整段 command 当路径
  if (name === "exec" || name === "shell_exec" || name === "operate" || name === "wait") {
    return "";
  }

  try {
    const parsed = JSON.parse(text) as unknown;
    if (typeof parsed === "string") {
      return looksLikeLocalPath(parsed) ? (normalizeLocalLinkHref(parsed) || parsed.trim()) : "";
    }
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      const record = parsed as Record<string, unknown>;
      const path = pickPathFromRecord(record);
      if (!path) return "";
      const offset = name === "read" || name === "read_file" ? readToolOffset(record) : undefined;
      return offset ? `${path}:${offset}` : path;
    }
  } catch {
    if (looksLikeLocalPath(text)) {
      return normalizeLocalLinkHref(text) || text;
    }
  }
  return "";
}

export type ToolCallResultStatus = {
  isDenied: boolean;
  isFailed: boolean;
  blockedReason?: string;
  message?: string;
};

export function parseToolCallResultStatus(resultText?: string): ToolCallResultStatus {
  const text = String(resultText || "").trim();
  if (!text) {
    return { isDenied: false, isFailed: false };
  }
  try {
    const data = JSON.parse(text);
    if (typeof data === "object" && data !== null) {
      const ok = data.ok;
      const approved = data.approved;
      const blockedReason = String(data.blockedReason || "");
      const isDenied = approved === false
        || blockedReason.includes("denied")
        || blockedReason === "rejected"
        || blockedReason.includes("refused");
      const isFailed = ok === false
        || isDenied
        || !!data.error
        || (typeof data.exitCode === "number" && data.exitCode !== 0);
      return {
        isDenied,
        isFailed,
        blockedReason: blockedReason || undefined,
        message: String(data.message || data.error || "").trim() || undefined,
      };
    }
  } catch {
    if (text.startsWith("Error:") || text.startsWith("error:")) {
      return { isDenied: false, isFailed: true, message: text };
    }
  }
  return { isDenied: false, isFailed: false };
}

export function buildToolcallPreviewMap(
  activityItems: ChatActivityItem[],
  noArgsText: string,
): Record<string, ToolcallPreviewEntry> {
  void noArgsText;
  const previews: Record<string, ToolcallPreviewEntry> = {};
  for (const item of activityItems) {
    if (item.kind !== "tool") continue;
    const toolCallId = String(item.toolCallId || "").trim();
    if (!toolCallId) continue;
    const name = String(item.name || "").trim();
    const filePath = extractToolcallFilePath(item.name, String(item.argsText || ""));
    const status = parseToolCallResultStatus(item.resultText);
    const title = status.isDenied ? `${name} (已拒绝)` : (status.isFailed ? `${name} (失败)` : name);
    previews[toolCallId] = {
      title,
      body: status.message || "",
      filePath: filePath || undefined,
      fileLabel: filePath || undefined,
    };
  }
  return previews;
}
