const WEB_ACCESS_PASSWORD_HASH_PREFIX = "sha256:";
const WEB_ACCESS_PASSWORD_HASH_STORAGE_PREFIX = "easy_call.web_access_password_hash.v1:";

export function normalizeWebAccessPasswordInput(value: string): string {
  return String(value || "")
    .trim()
    .split("")
    .filter((ch) => /[A-Za-z0-9]/.test(ch))
    .join("")
    .toUpperCase();
}

export async function hashWebAccessPassword(value: string): Promise<string> {
  const normalized = normalizeWebAccessPasswordInput(value);
  if (!normalized) return "";
  if (typeof window === "undefined" || !window.crypto?.subtle) {
    throw new Error("当前环境不支持 Web Crypto");
  }
  const bytes = new TextEncoder().encode(normalized);
  const digest = await window.crypto.subtle.digest("SHA-256", bytes);
  const hex = Array.from(new Uint8Array(digest))
    .map((item) => item.toString(16).padStart(2, "0"))
    .join("");
  return `${WEB_ACCESS_PASSWORD_HASH_PREFIX}${hex}`;
}

function webAccessPasswordHashStorageKey(chatUrl: string): string {
  return `${WEB_ACCESS_PASSWORD_HASH_STORAGE_PREFIX}${String(chatUrl || "").trim()}`;
}

export function readRememberedWebAccessPasswordHash(chatUrl: string): string {
  if (typeof window === "undefined") return "";
  return String(window.localStorage.getItem(webAccessPasswordHashStorageKey(chatUrl)) || "").trim();
}

export function rememberWebAccessPasswordHash(chatUrl: string, passwordHash: string) {
  if (typeof window === "undefined") return;
  const normalizedChatUrl = String(chatUrl || "").trim();
  if (!normalizedChatUrl) return;
  const normalizedHash = String(passwordHash || "").trim();
  if (!normalizedHash) {
    window.localStorage.removeItem(webAccessPasswordHashStorageKey(normalizedChatUrl));
    return;
  }
  window.localStorage.setItem(webAccessPasswordHashStorageKey(normalizedChatUrl), normalizedHash);
}

export function clearRememberedWebAccessPasswordHash(chatUrl: string) {
  rememberWebAccessPasswordHash(chatUrl, "");
}
