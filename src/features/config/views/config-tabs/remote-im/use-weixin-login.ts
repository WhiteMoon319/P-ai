import { computed, onScopeDispose, ref, type ComputedRef, type Ref } from "vue";
import { invokeTauri } from "../../../../../services/tauri-api";
import type { RemoteImChannelConfig } from "../../../../../types/app";
import type { ChannelConnectionStatus, WeixinLoginStatus } from "./types";

type Translate = (key: string, params?: Record<string, unknown>) => string;

type UseWeixinLoginOptions = {
  selectedChannel: ComputedRef<RemoteImChannelConfig | null>;
  channelRuntimeStates: Ref<Record<string, ChannelConnectionStatus | null>>;
  channelDirty: ComputedRef<boolean>;
  saveChannels: () => Promise<boolean>;
  refreshChannelStatus: () => Promise<void>;
  refreshContacts: () => Promise<void>;
  setStatus: (text: string) => void;
  t: Translate;
};

function emptyWeixinLoginStatus(channelId: string): WeixinLoginStatus {
  return {
    channelId,
    connected: false,
    status: "",
    message: "",
    sessionKey: "",
    qrcode: "",
    qrcodeImgContent: "",
    accountId: "",
    userId: "",
    baseUrl: "",
    lastError: "",
  };
}

function looksLikeBase64(value: string): boolean {
  if (!value || value.length < 64) return false;
  return /^[A-Za-z0-9+/=]+$/.test(value);
}

export function useWeixinLogin(options: UseWeixinLoginOptions) {
  const weixinLoginStates = ref<Record<string, WeixinLoginStatus | null>>({});
  const weixinLoginBusy = ref(false);
  let loginPollTimer: ReturnType<typeof setInterval> | null = null;

  const weixinLoginState = computed(() => {
    const channelId = options.selectedChannel.value?.id || "";
    return weixinLoginStates.value[channelId] || emptyWeixinLoginStatus(channelId);
  });

  const weixinQrImageSrc = computed(() => {
    const raw = String(weixinLoginState.value.qrcodeImgContent || "").trim();
    if (!raw) return "";
    if (raw.startsWith("data:image/")) return raw;
    if (/^https?:\/\//i.test(raw)) {
      return `https://api.qrserver.com/v1/create-qr-code/?size=384x384&margin=0&data=${encodeURIComponent(raw)}`;
    }
    if (looksLikeBase64(raw)) {
      return `data:image/png;base64,${raw}`;
    }
    return raw;
  });

  const persistedWeixinCredentials = computed(() => {
    const creds = options.selectedChannel.value?.credentials;
    if (!creds || typeof creds !== "object") {
      return { token: "", accountId: "" };
    }
    const record = creds as Record<string, unknown>;
    return {
      token: String(record.token || "").trim(),
      accountId: String(record.accountId || "").trim(),
    };
  });

  const weixinRuntimeStatus = computed(() => {
    const channelId = options.selectedChannel.value?.id;
    return channelId ? options.channelRuntimeStates.value[channelId] ?? null : null;
  });

  const isWeixinLoggedIn = computed(() => {
    const status = String(weixinLoginState.value.status || "").trim().toLowerCase();
    if (weixinLoginState.value.connected || weixinRuntimeStatus.value?.connected) return true;
    if (status === "confirmed" || status === "logged_in") return true;
    if (String(weixinLoginState.value.accountId || "").trim()) return true;
    if (persistedWeixinCredentials.value.token) return true;
    return !!persistedWeixinCredentials.value.accountId;
  });

  const weixinStatusText = computed(() => {
    if (weixinRuntimeStatus.value?.connected) return options.t("config.remoteIm.weixinConnected");
    if (isWeixinLoggedIn.value) return options.t("config.remoteIm.weixinLoggedIn");
    const status = String(weixinLoginState.value.status || "").trim().toLowerCase();
    if (status === "wait" || status === "scanned" || status === "scaned") {
      return options.t("config.remoteIm.waitingScanConfirm");
    }
    if (status === "confirmed" || status === "logged_in") {
      return options.t("config.remoteIm.weixinLoggedIn");
    }
    return options.t("config.remoteIm.waitingScan");
  });

  const weixinStatusMessage = computed(() => {
    if (weixinRuntimeStatus.value?.connected || isWeixinLoggedIn.value) {
      return options.t("config.remoteIm.credentialsSaved");
    }
    const status = String(weixinLoginState.value.status || "").trim().toLowerCase();
    if (status === "wait" || status === "scanned" || status === "scaned") {
      return options.t("config.remoteIm.confirmLoginInWeixin");
    }
    return String(weixinLoginState.value.lastError || "").trim();
  });

  function stopLoginPolling() {
    if (!loginPollTimer) return;
    clearInterval(loginPollTimer);
    loginPollTimer = null;
  }

  async function pollWeixinLoginStatus() {
    const channel = options.selectedChannel.value;
    if (!channel || channel.platform !== "weixin_oc") return;
    const channelId = channel.id;
    try {
      const result = await invokeTauri<WeixinLoginStatus>("remote_im_weixin_oc_get_login_status", {
        input: { channelId },
      });
      weixinLoginStates.value = { ...weixinLoginStates.value, [channelId]: result };
      if (result.connected || result.status === "expired") {
        stopLoginPolling();
        if (result.connected) {
          await options.refreshChannelStatus();
          await options.refreshContacts();
        }
      }
    } catch (error) {
      const errMsg = options.t("config.remoteIm.weixinStatusQueryFailed", { error: String(error) });
      weixinLoginStates.value = {
        ...weixinLoginStates.value,
        [channelId]: {
          ...(weixinLoginStates.value[channelId] || emptyWeixinLoginStatus(channelId)),
          message: errMsg,
          lastError: errMsg,
        },
      };
      options.setStatus(errMsg);
    }
  }

  async function startWeixinLogin() {
    const channel = options.selectedChannel.value;
    if (!channel || channel.platform !== "weixin_oc") return;
    weixinLoginBusy.value = true;
    try {
      const result = await invokeTauri<WeixinLoginStatus>("remote_im_weixin_oc_start_login", {
        input: { channelId: channel.id, forceRefresh: true },
      });
      weixinLoginStates.value = {
        ...weixinLoginStates.value,
        [channel.id]: {
          channelId: result.channelId,
          connected: false,
          status: result.status,
          message: result.message,
          sessionKey: result.sessionKey,
          qrcode: result.qrcode,
          qrcodeImgContent: result.qrcodeImgContent,
          accountId: "",
          userId: "",
          baseUrl: "",
          lastError: "",
        },
      };
      stopLoginPolling();
      loginPollTimer = setInterval(() => void pollWeixinLoginStatus(), 2500);
    } catch (error) {
      options.setStatus(options.t("config.remoteIm.weixinScanLoginFailed", { error: String(error) }));
    } finally {
      weixinLoginBusy.value = false;
    }
  }

  async function logoutWeixin() {
    const channel = options.selectedChannel.value;
    if (!channel || channel.platform !== "weixin_oc") return;
    try {
      await invokeTauri<boolean>("remote_im_weixin_oc_logout", { input: { channelId: channel.id } });
      weixinLoginStates.value = {
        ...weixinLoginStates.value,
        [channel.id]: {
          channelId: channel.id,
          connected: false,
          status: "logged_out",
          message: options.t("config.remoteIm.loggedOut"),
        },
      };
      await options.refreshChannelStatus();
      options.setStatus(options.t("config.remoteIm.weixinLoggedOut"));
    } catch (error) {
      options.setStatus(options.t("config.remoteIm.weixinLogoutFailed", { error: String(error) }));
    }
  }

  async function onWeixinLoginButtonClick() {
    if (weixinLoginBusy.value) return;
    if (options.channelDirty.value) {
      options.setStatus(options.t("config.remoteIm.savingWeixinConfig"));
      if (!(await options.saveChannels())) {
        options.setStatus(options.t("config.remoteIm.saveWeixinFirst"));
        return;
      }
    }
    if (isWeixinLoggedIn.value) await logoutWeixin();
    await startWeixinLogin();
  }

  async function syncWeixinContacts() {
    const channel = options.selectedChannel.value;
    if (!channel || channel.platform !== "weixin_oc") return;
    try {
      const result = await invokeTauri<{ message: string }>("remote_im_weixin_oc_sync_contacts", {
        input: { channelId: channel.id },
      });
      options.setStatus(result.message);
      await options.refreshContacts();
    } catch (error) {
      options.setStatus(options.t("config.remoteIm.weixinContactSyncFailed", { error: String(error) }));
    }
  }

  onScopeDispose(stopLoginPolling);

  return {
    isWeixinLoggedIn,
    onWeixinLoginButtonClick,
    syncWeixinContacts,
    weixinLoginBusy,
    weixinLoginState,
    weixinQrImageSrc,
    weixinStatusMessage,
    weixinStatusText,
  };
}
