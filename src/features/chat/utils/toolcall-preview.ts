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
      return pickPathFromRecord(parsed as Record<string, unknown>);
    }
  } catch {
    if (looksLikeLocalPath(text)) {
      return normalizeLocalLinkHref(text) || text;
    }
  }
  return "";
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
    const title = String(item.name || "").trim();
    const filePath = extractToolcallFilePath(item.name, String(item.argsText || ""));
    previews[toolCallId] = {
      title,
      body: "",
      filePath: filePath || undefined,
      fileLabel: filePath || undefined,
    };
  }
  return previews;
}
