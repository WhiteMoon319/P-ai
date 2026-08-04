// ==================== 远程前端模式 ====================
// 手机 PAI 作为电脑 PAI 前端：远程目标与激活状态存 localStorage
// （Android 单 WebView 内 chat/settings 页面同 origin，localStorage 共享）。
// 应用重启默认本地模式（active 标志不持久，仅目标保留）。

import { computed, ref } from "vue";

const REMOTE_TARGET_STORAGE_KEY = "pai.remote_frontend.target.v1";
const REMOTE_ACTIVE_STORAGE_KEY = "pai.remote_frontend.active.v1";

export const DEFAULT_REMOTE_PORT = 8429;

export type RemoteTarget = {
  host: string;
  port: number;
  password?: string;
};

export type RemoteView = "chat" | "settings";

function readRemoteTarget(): RemoteTarget | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(REMOTE_TARGET_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { host?: unknown; port?: unknown; password?: unknown };
    const host = String(parsed?.host || "").trim();
    const port = Number(parsed?.port);
    if (!host || !Number.isInteger(port) || port < 1 || port > 65535) return null;
    const password = typeof parsed?.password === "string" ? parsed.password : "";
    return { host, port, password: password || undefined };
  } catch {
    return null;
  }
}

function writeRemoteTarget(target: RemoteTarget | null): void {
  if (typeof window === "undefined") return;
  if (target) {
    window.localStorage.setItem(REMOTE_TARGET_STORAGE_KEY, JSON.stringify(target));
  } else {
    window.localStorage.removeItem(REMOTE_TARGET_STORAGE_KEY);
  }
}

function readRemoteActive(): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(REMOTE_ACTIVE_STORAGE_KEY) === "1";
}

function writeRemoteActive(active: boolean): void {
  if (typeof window === "undefined") return;
  if (active) {
    window.localStorage.setItem(REMOTE_ACTIVE_STORAGE_KEY, "1");
  } else {
    window.localStorage.removeItem(REMOTE_ACTIVE_STORAGE_KEY);
  }
}

export function buildRemoteUrl(target: RemoteTarget, view: RemoteView): string {
  const path = view === "settings" ? "settings" : "sidebar";
  return `http://${target.host}:${target.port}/${path}`;
}

/** 校验连接表单输入，返回规范化远程目标；非法返回 null。 */
export function parseRemoteTargetInput(host: string, port: string): RemoteTarget | null {
  const normalizedHost = String(host || "")
    .trim()
    .replace(/^https?:\/\//i, "")
    .replace(/\/+$/, "");
  const normalizedPort = String(port || "").trim();
  if (!normalizedHost || !normalizedPort) return null;
  const portNumber = Number(normalizedPort);
  if (!Number.isInteger(portNumber) || portNumber < 1 || portNumber > 65535) return null;
  if (!/^[\w.-]+$/.test(normalizedHost)) return null;
  return { host: normalizedHost, port: portNumber };
}

export function useRemoteMode() {
  const remoteActive = ref(readRemoteActive());
  const remoteTarget = ref<RemoteTarget | null>(readRemoteTarget());
  const remoteView = ref<RemoteView>("chat");

  const isRemoteMode = computed(() => remoteActive.value && !!remoteTarget.value);
  const remoteUrl = computed(() => {
    const target = remoteTarget.value;
    if (!target) return "";
    return buildRemoteUrl(target, remoteView.value);
  });
  const remoteTargetText = computed(() => {
    const target = remoteTarget.value;
    if (!target) return "";
    return `${target.host}:${target.port}`;
  });

  function enterRemote(target: RemoteTarget) {
    remoteTarget.value = target;
    remoteActive.value = true;
    remoteView.value = "chat";
    writeRemoteTarget(target);
    writeRemoteActive(true);
  }

  function exitRemote() {
    remoteActive.value = false;
    remoteView.value = "chat";
    writeRemoteActive(false);
  }

  function setRemoteView(view: RemoteView) {
    remoteView.value = view;
  }

  return {
    remoteActive,
    remoteTarget,
    remoteView,
    isRemoteMode,
    remoteUrl,
    remoteTargetText,
    enterRemote,
    exitRemote,
    setRemoteView,
  };
}
