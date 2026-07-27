function safeDecodeUriComponent(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export function normalizeLocalLinkHref(href: string): string {
  const trimmed = String(href || "").trim();
  if (!trimmed) return "";

  if (/^file:/i.test(trimmed)) {
    try {
      const url = new URL(trimmed);
      const decodedPath = safeDecodeUriComponent(url.pathname || "");
      if (url.host && url.host !== "localhost") {
        return `\\\\${url.host}${decodedPath.replace(/\//g, "\\")}`;
      }
      const windowsPath = decodedPath.replace(/^\/([A-Za-z]:)/, "$1");
      return windowsPath.replace(/\\/g, "/");
    } catch {
      return safeDecodeUriComponent(trimmed);
    }
  }

  const decoded = safeDecodeUriComponent(trimmed).replace(/%5C/gi, "\\");
  // /E:/path → E:/path (Windows 路径带前导斜杠)
  const windowsNormalized = decoded.replace(/^\/([A-Za-z]:)/, "$1");
  if (/^[A-Za-z]:[\\/]/.test(windowsNormalized)) {
    return `${windowsNormalized.slice(0, 2)}${windowsNormalized.slice(2).replace(/\\/g, "/")}`;
  }
  return decoded;
}

export function isAbsoluteLocalPath(href: string): boolean {
  const normalized = normalizeLocalLinkHref(href);
  return /^[A-Za-z]:[\\/]/.test(normalized) || normalized.startsWith("\\\\") || normalized.startsWith("/");
}

export const ASSISTANT_SPACE_PATH_PREFIX = "{Assistant Space}";

export function isAssistantSpacePath(value: string): boolean {
  const normalized = String(value || "").trim().replace(/\\/g, "/");
  return normalized === ASSISTANT_SPACE_PATH_PREFIX
    || normalized.startsWith(`${ASSISTANT_SPACE_PATH_PREFIX}/`);
}

export function normalizeAssistantSpacePath(value: string): string {
  const normalized = String(value || "").trim().replace(/\\/g, "/");
  if (!isAssistantSpacePath(normalized)) return normalized;
  const suffix = normalized.slice(ASSISTANT_SPACE_PATH_PREFIX.length).replace(/^\/+/, "");
  return suffix ? `${ASSISTANT_SPACE_PATH_PREFIX}/${suffix}` : ASSISTANT_SPACE_PATH_PREFIX;
}

export type LocalFileReference = {
  path: string;
  line?: number;
  column?: number;
};

export function parseLocalFileReference(href: string): LocalFileReference | null {
  const normalized = normalizeLocalLinkHref(href);
  if (!normalized) return null;
  const match = normalized.match(/^(.*?)(?::(\d+))(?::(\d+))?$/);
  const path = String(match ? match[1] : normalized).trim();
  if (!path) return null;
  return {
    path,
    line: match?.[2] ? Math.max(1, Number.parseInt(match[2], 10)) : undefined,
    column: match?.[3] ? Math.max(1, Number.parseInt(match[3], 10)) : undefined,
  };
}
