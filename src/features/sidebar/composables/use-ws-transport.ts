import { computed, onBeforeUnmount, ref } from "vue";

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
  timer: number;
};

export type SidebarBridgeConfig = {
  chatUrl: string;
  token?: string;
};

type PendingAttachmentChunk = {
  resolve: (value: { transferId: string; nextOffset: number }) => void;
  reject: (reason?: unknown) => void;
  timer: number;
};

export type SidebarAttachmentReceipt = {
  id: string;
  fileName: string;
  mime: string;
  size: number;
  path: string;
  attachAsMedia: boolean;
  textNotice: string;
  previewDataUrl?: string;
};

const SIDEBAR_BRIDGE_TOKEN_STORAGE_PREFIX = "easy_call.sidebar.bridge_token.v1:";

function sidebarBridgeTokenStorageKey(chatUrl: string): string {
  return `${SIDEBAR_BRIDGE_TOKEN_STORAGE_PREFIX}${chatUrl.trim()}`;
}

function readPersistedSidebarBridgeToken(chatUrl: string): string {
  if (typeof window === "undefined") return "";
  return String(window.localStorage.getItem(sidebarBridgeTokenStorageKey(chatUrl)) || "").trim();
}

function persistSidebarBridgeToken(chatUrl: string, token: string) {
  if (typeof window === "undefined") return;
  const normalizedChatUrl = String(chatUrl || "").trim();
  if (!normalizedChatUrl) return;
  const normalizedToken = String(token || "").trim();
  if (!normalizedToken) {
    window.localStorage.removeItem(sidebarBridgeTokenStorageKey(normalizedChatUrl));
    return;
  }
  window.localStorage.setItem(sidebarBridgeTokenStorageKey(normalizedChatUrl), normalizedToken);
}

function clearPersistedSidebarBridgeToken(chatUrl: string) {
  persistSidebarBridgeToken(chatUrl, "");
}

export function useWsTransport() {
  const socket = ref<WebSocket | null>(null);
  const connected = ref(false);
  const connecting = ref(false);
  const bridgeReady = ref(false);
  const authRequired = ref(false);
  const authenticated = ref(true);
  const errorText = ref("");
  const bridgeConfig = ref<SidebarBridgeConfig | null>(null);
  const notificationHandlers = new Map<string, Set<(payload: unknown) => void>>();
  const pending = new Map<number, PendingRequest>();
  const pendingAttachmentChunks = new Map<string, PendingAttachmentChunk>();
  let authRefreshHandler: (() => void) | null = null;
  let requestId = 1;

  const canSend = computed(() => connected.value && socket.value?.readyState === WebSocket.OPEN);

  function emitNotification(method: string, payload: unknown) {
    const handlers = notificationHandlers.get(method);
    if (!handlers) return;
    for (const handler of handlers) handler(payload);
  }

  function settle(id: number, payload: Record<string, unknown>) {
    const item = pending.get(id);
    if (!item) return;
    pending.delete(id);
    window.clearTimeout(item.timer);
    if (payload.error) {
      const error = payload.error as { message?: string };
      const message = String(error?.message || "请求失败");
      if (message.includes("token expired") || message.includes("discovery refreshed") || message.includes("invalid authToken")) {
        const currentChatUrl = String(bridgeConfig.value?.chatUrl || "").trim();
        if (currentChatUrl) {
          clearPersistedSidebarBridgeToken(currentChatUrl);
        }
        if (bridgeConfig.value) {
          bridgeConfig.value = { ...bridgeConfig.value, token: undefined };
        }
        authRefreshHandler?.();
      }
      item.reject(new Error(message));
      return;
    }
    if (payload.result && typeof payload.result === "object" && (payload.result as { authenticated?: unknown }).authenticated === true) {
      const authToken = String((payload.result as { authToken?: unknown }).authToken || "").trim();
      const currentChatUrl = String(bridgeConfig.value?.chatUrl || "").trim();
      if (authToken && currentChatUrl) {
        persistSidebarBridgeToken(currentChatUrl, authToken);
        if (bridgeConfig.value) {
          bridgeConfig.value = { ...bridgeConfig.value, token: authToken };
        }
      }
      authenticated.value = true;
      authRequired.value = false;
    }
    item.resolve(payload.result);
  }

  function settleAttachmentChunk(payload: Record<string, unknown>) {
    const params = (payload.params || {}) as { transferId?: unknown; nextOffset?: unknown };
    const transferId = String(params.transferId || "").trim();
    if (payload.method === "attachment.chunkAck" && transferId) {
      const pendingChunk = pendingAttachmentChunks.get(transferId);
      if (!pendingChunk) return;
      pendingAttachmentChunks.delete(transferId);
      window.clearTimeout(pendingChunk.timer);
      const nextOffset = Number(params.nextOffset);
      if (!Number.isSafeInteger(nextOffset) || nextOffset < 0) {
        pendingChunk.reject(new Error("附件分块确认 offset 无效"));
        return;
      }
      pendingChunk.resolve({ transferId, nextOffset });
      return;
    }
    if (payload.error && pendingAttachmentChunks.size > 0) {
      const error = payload.error as { message?: unknown };
      const reason = new Error(String(error?.message || "附件分块传输失败"));
      for (const [pendingTransferId, pendingChunk] of pendingAttachmentChunks.entries()) {
        pendingAttachmentChunks.delete(pendingTransferId);
        window.clearTimeout(pendingChunk.timer);
        pendingChunk.reject(reason);
      }
    }
  }

  function handleMessage(event: MessageEvent<string>, ready?: () => void) {
    let payload: Record<string, unknown>;
    try {
      payload = JSON.parse(String(event.data || "{}"));
    } catch {
      return;
    }
    if (typeof payload.id === "number") {
      settle(payload.id, payload);
      return;
    }
    const method = String(payload.method || "");
    if (method === "attachment.chunkAck") {
      settleAttachmentChunk(payload);
      return;
    }
    if (payload.error) {
      settleAttachmentChunk(payload);
    }
    if (method === "bridge.ready") {
      const params = (payload.params || {}) as { authRequired?: unknown };
      const hasAuthToken = !!String(bridgeConfig.value?.token || "").trim();
      bridgeReady.value = true;
      authRequired.value = !!params.authRequired;
      authenticated.value = !authRequired.value || hasAuthToken;
      ready?.();
    }
    if (method) emitNotification(method, payload.params);
  }

  function close() {
    const current = socket.value;
    socket.value = null;
    connected.value = false;
    connecting.value = false;
    bridgeReady.value = false;
    authRequired.value = false;
    authenticated.value = true;
    for (const [id, item] of pending.entries()) {
      window.clearTimeout(item.timer);
      item.reject(new Error("连接已断开"));
      pending.delete(id);
    }
    for (const [transferId, item] of pendingAttachmentChunks.entries()) {
      window.clearTimeout(item.timer);
      item.reject(new Error("连接已断开"));
      pendingAttachmentChunks.delete(transferId);
    }
    if (current && current.readyState !== WebSocket.CLOSED) current.close();
  }

  async function connect(config: SidebarBridgeConfig) {
    close();
    const persistedToken = config.token ? "" : readPersistedSidebarBridgeToken(config.chatUrl);
    const nextConfig: SidebarBridgeConfig = {
      ...config,
      token: String(config.token || persistedToken || "").trim() || undefined,
    };
    bridgeConfig.value = nextConfig;
    connecting.value = true;
    bridgeReady.value = false;
    authRequired.value = false;
    authenticated.value = true;
    errorText.value = "";
    await new Promise<void>((resolve, reject) => {
      let settled = false;
      let readyTimer: number | null = null;
      const finishReady = () => {
        if (settled) return;
        settled = true;
        if (readyTimer !== null) window.clearTimeout(readyTimer);
        connected.value = true;
        connecting.value = false;
        resolve();
      };
      const fail = (error: unknown) => {
        if (readyTimer !== null) window.clearTimeout(readyTimer);
        connected.value = false;
        connecting.value = false;
        if (socket.value?.readyState !== WebSocket.OPEN) {
          bridgeReady.value = false;
        }
        errorText.value = String(error || "PAI 未运行");
        if (!settled) {
          settled = true;
          reject(error instanceof Error ? error : new Error(String(error || "PAI 未运行")));
        }
      };
      const ws = new WebSocket(nextConfig.chatUrl);
      socket.value = ws;
      ws.onopen = () => {
        connected.value = true;
        readyTimer = window.setTimeout(() => {
          if (!bridgeReady.value) fail(new Error("等待 PAI 侧边栏桥接就绪超时"));
        }, 5000);
      };
      ws.onerror = () => {
        fail(new Error("PAI 未运行"));
      };
      ws.onclose = () => {
        for (const [id, item] of pending.entries()) {
          window.clearTimeout(item.timer);
          item.reject(new Error("连接已断开"));
          pending.delete(id);
        }
        for (const [transferId, item] of pendingAttachmentChunks.entries()) {
          window.clearTimeout(item.timer);
          item.reject(new Error("连接已断开"));
          pendingAttachmentChunks.delete(transferId);
        }
        if (!settled) {
          fail(new Error("PAI 未运行"));
        } else if (socket.value === ws) {
          connected.value = false;
          connecting.value = false;
          bridgeReady.value = false;
          authRequired.value = false;
          authenticated.value = true;
          errorText.value = "连接已断开";
        }
      };
      ws.onmessage = (event) => handleMessage(event, finishReady);
    });
  }

  function request<T>(method: string, params: Record<string, unknown> = {}, timeoutMs = 30000): Promise<T> {
    const currentSocket = socket.value;
    if (!canSend.value || !currentSocket || currentSocket.readyState !== WebSocket.OPEN) {
      return Promise.reject(new Error("PAI 未运行"));
    }
    if (authRequired.value && !authenticated.value && method !== "auth.login") {
      return Promise.reject(new Error("远程访问需要先输入密码"));
    }
    const id = requestId++;
    const authToken = String(bridgeConfig.value?.token || "").trim();
    const bodyParams = authToken ? { authToken, ...params } : params;
    const body = { jsonrpc: "2.0", id, method, params: bodyParams };
    return new Promise<T>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        pending.delete(id);
        reject(new Error("请求超时"));
      }, timeoutMs);
      pending.set(id, { resolve: resolve as (value: unknown) => void, reject, timer });
      try {
        currentSocket.send(JSON.stringify(body));
      } catch (error) {
        pending.delete(id);
        window.clearTimeout(timer);
        reject(error);
      }
    });
  }

  function onNotification(method: string, handler: (payload: unknown) => void) {
    const handlers = notificationHandlers.get(method) || new Set<(payload: unknown) => void>();
    handlers.add(handler);
    notificationHandlers.set(method, handlers);
    return () => handlers.delete(handler);
  }

  function onAuthRefreshNeeded(handler: () => void) {
    authRefreshHandler = handler;
  }

  function uuidToBytes(value: string): Uint8Array {
    const normalized = String(value || "").replace(/-/g, "").trim();
    if (!/^[0-9a-f]{32}$/i.test(normalized)) {
      throw new Error("附件传输 ID 无效");
    }
    const bytes = new Uint8Array(16);
    for (let index = 0; index < 16; index += 1) {
      bytes[index] = Number.parseInt(normalized.slice(index * 2, index * 2 + 2), 16);
    }
    return bytes;
  }

  function waitForAttachmentChunk(
    transferId: string,
    timeoutMs = 30000,
  ): Promise<{ transferId: string; nextOffset: number }> {
    return new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        pendingAttachmentChunks.delete(transferId);
        reject(new Error("附件分块确认超时"));
      }, timeoutMs);
      pendingAttachmentChunks.set(transferId, { resolve, reject, timer });
    });
  }

  function clearPendingAttachmentChunk(transferId: string) {
    const pendingChunk = pendingAttachmentChunks.get(transferId);
    if (!pendingChunk) return;
    pendingAttachmentChunks.delete(transferId);
    window.clearTimeout(pendingChunk.timer);
  }

  async function uploadAttachment(file: File): Promise<SidebarAttachmentReceipt> {
    const size = Number(file.size || 0);
    const maxBytes = 50 * 1024 * 1024;
    if (size > maxBytes) {
      const error = new Error("FILE_TOO_LARGE: 文件太大，单个文件不能超过 50 MiB") as Error & { code?: string };
      error.code = "FILE_TOO_LARGE";
      throw error;
    }
    const begin = await request<{ transferId: string; nextOffset: number; chunkSize?: number }>(
      "attachment.transfer.begin",
      {
        fileName: String(file.name || "attachment").trim() || "attachment",
        mime: String(file.type || "").trim(),
        size,
      },
      30000,
    );
    const transferId = String(begin?.transferId || "").trim();
    if (!transferId) throw new Error("附件传输未返回 transferId");
    const chunkSize = Math.max(1, Math.min(Number(begin?.chunkSize || 256 * 1024), 256 * 1024));
    let offset = Number(begin?.nextOffset || 0);
    try {
      while (offset < size) {
        const end = Math.min(size, offset + chunkSize);
        const chunk = new Uint8Array(await file.slice(offset, end).arrayBuffer());
        if (chunk.length === 0) throw new Error("附件分块为空");
        const frame = new Uint8Array(29 + chunk.length);
        frame[0] = 1;
        frame.set(uuidToBytes(transferId), 1);
        const frameView = new DataView(frame.buffer);
        frameView.setUint32(17, Math.floor(offset / 0x100000000), false);
        frameView.setUint32(21, offset >>> 0, false);
        frameView.setUint32(25, chunk.length, false);
        frame.set(chunk, 29);
        let ack: { transferId: string; nextOffset: number } | null = null;
        for (let attempt = 0; attempt < 2; attempt += 1) {
          const ackPromise = waitForAttachmentChunk(transferId);
          try {
            if (!socket.value || socket.value.readyState !== WebSocket.OPEN) {
              throw new Error("连接已断开");
            }
            socket.value.send(frame);
            ack = await ackPromise;
            break;
          } catch (error) {
            clearPendingAttachmentChunk(transferId);
            if (attempt === 1) throw error;
          }
        }
        if (!ack) throw new Error("附件分块传输失败");
        if (ack.nextOffset <= offset || ack.nextOffset > size) {
          throw new Error(`附件分块确认 offset 无效：${ack.nextOffset}`);
        }
        offset = ack.nextOffset;
      }
      return await request<SidebarAttachmentReceipt>("attachment.transfer.complete", { transferId }, 60000);
    } catch (error) {
      try {
        await request("attachment.transfer.abort", { transferId }, 5000);
      } catch {
        // 连接断开或会话已清理时无需重复报告 abort 错误。
      }
      throw error;
    }
  }

  async function login(password: string): Promise<void> {
    const normalizedPassword = String(password || "").trim();
    await request("auth.login", {
      ...(normalizedPassword ? { password: normalizedPassword } : {}),
    }, 10000);
  }

  async function reconnect() {
    const config = bridgeConfig.value;
    if (!config) return;
    await connect(config);
  }

  async function ping(timeoutMs = 2500): Promise<void> {
    await request("bridge.ping", {}, timeoutMs);
  }

  onBeforeUnmount(() => close());

  return {
    connected,
    connecting,
    bridgeReady,
    authRequired,
    authenticated,
    errorText,
    bridgeConfig,
    canSend,
    connect,
    reconnect,
    close,
    login,
    request,
    uploadAttachment,
    ping,
    onNotification,
    onAuthRefreshNeeded,
  };
}
