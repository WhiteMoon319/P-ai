import { computed, effectScope, ref } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RemoteImChannelConfig } from "../../../../../types/app";
import { invokeTauri } from "../../../../../services/tauri-api";
import { useWeixinLogin } from "./use-weixin-login";

vi.mock("../../../../../services/tauri-api", () => ({
  invokeTauri: vi.fn(),
}));

const mockedInvokeTauri = vi.mocked(invokeTauri);

function createChannel(): RemoteImChannelConfig {
  return {
    id: "weixin-1",
    name: "微信",
    platform: "weixin_oc",
    enabled: true,
    activateAssistant: true,
    receiveFiles: true,
    streamingSend: false,
    showToolCalls: false,
    filterMarkdown: false,
    allowSendFiles: false,
    credentials: {},
  };
}

function createHarness(options?: { dirty?: boolean; saveResult?: boolean }) {
  const scope = effectScope();
  const channel = ref<RemoteImChannelConfig | null>(createChannel());
  const dirty = ref(!!options?.dirty);
  const saveChannels = vi.fn(async () => options?.saveResult ?? true);
  const refreshChannelStatus = vi.fn(async () => undefined);
  const refreshContacts = vi.fn(async () => undefined);
  const setStatus = vi.fn();
  const api = scope.run(() => useWeixinLogin({
    selectedChannel: computed(() => channel.value),
    channelRuntimeStates: ref({}),
    channelDirty: computed(() => dirty.value),
    saveChannels,
    refreshChannelStatus,
    refreshContacts,
    setStatus,
    t: (key) => key,
  }));
  if (!api) throw new Error("微信登录测试运行时初始化失败");
  return { api, refreshChannelStatus, refreshContacts, saveChannels, scope, setStatus };
}

describe("useWeixinLogin", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockedInvokeTauri.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("配置保存失败时不启动扫码登录", async () => {
    const harness = createHarness({ dirty: true, saveResult: false });
    await harness.api.onWeixinLoginButtonClick();

    expect(harness.saveChannels).toHaveBeenCalledOnce();
    expect(mockedInvokeTauri).not.toHaveBeenCalled();
    expect(harness.setStatus).toHaveBeenLastCalledWith("config.remoteIm.saveWeixinFirst");
    harness.scope.stop();
  });

  it("扫码登录后轮询成功会刷新渠道状态和联系人", async () => {
    mockedInvokeTauri
      .mockResolvedValueOnce({
        channelId: "weixin-1",
        status: "wait",
        message: "",
        sessionKey: "session-1",
        qrcode: "qr",
        qrcodeImgContent: "qr-content",
      })
      .mockResolvedValueOnce({
        channelId: "weixin-1",
        connected: true,
        status: "logged_in",
        message: "ok",
      });
    const harness = createHarness();

    await harness.api.onWeixinLoginButtonClick();
    expect(harness.api.weixinLoginState.value.connected).toBe(false);
    expect(harness.api.weixinLoginState.value.sessionKey).toBe("session-1");

    await vi.advanceTimersByTimeAsync(2500);
    expect(harness.refreshChannelStatus).toHaveBeenCalledOnce();
    expect(harness.refreshContacts).toHaveBeenCalledOnce();
    harness.scope.stop();
  });

  it("同步联系人沿用当前渠道并刷新正式联系人列表", async () => {
    mockedInvokeTauri.mockResolvedValueOnce({ message: "synced" });
    const harness = createHarness();

    await harness.api.syncWeixinContacts();

    expect(mockedInvokeTauri).toHaveBeenCalledWith("remote_im_weixin_oc_sync_contacts", {
      input: { channelId: "weixin-1" },
    });
    expect(harness.setStatus).toHaveBeenCalledWith("synced");
    expect(harness.refreshContacts).toHaveBeenCalledOnce();
    harness.scope.stop();
  });
});
