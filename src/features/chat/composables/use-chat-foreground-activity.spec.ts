import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { ref } from "vue";
import { useChatForegroundActivity } from "./use-chat-foreground-activity";

const restoreMock = vi.hoisted(() => vi.fn(async () => {}));
const setActiveMock = vi.hoisted(() => vi.fn(async () => {}));

vi.mock("../../../services/tauri-api", () => ({
  restoreTransportAfterForegroundWake: restoreMock,
  setTransportChatViewActive: setActiveMock,
}));

function stubDocument(visibilityState: string, hasFocus: boolean) {
  Object.defineProperty(globalThis, "document", {
    value: { visibilityState, hasFocus: () => hasFocus },
    writable: true,
    configurable: true,
  });
}

async function flush() {
  for (let i = 0; i < 10; i += 1) await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 10));
}

describe("useChatForegroundActivity", () => {
  beforeEach(() => {
    restoreMock.mockReset().mockImplementation(async () => {});
    setActiveMock.mockReset().mockImplementation(async () => {});
    vi.useRealTimers();
  });

  afterEach(() => {
    // @ts-expect-error 测试清理
    delete globalThis.document;
  });

  it("可见但无获焦仍唤醒：移动端第一次回来就能恢复", async () => {
    stubDocument("visible", false);
    const activeSynced = ref<boolean | null>(null);
    const onWake = vi.fn(async () => {});
    const activity = useChatForegroundActivity({ activeSynced, onWake });

    await activity.sync("visibilitychange");
    await flush();

    expect(onWake).toHaveBeenCalledTimes(1);
    expect(activeSynced.value).toBe(true);
    activity.cleanup();
  });

  it("可见内连击不重复唤醒：focus 到达时已同步则空转", async () => {
    stubDocument("visible", false);
    const activeSynced = ref<boolean | null>(null);
    const onWake = vi.fn(async () => {});
    const activity = useChatForegroundActivity({ activeSynced, onWake });

    await activity.sync("visibilitychange");
    await flush();
    expect(onWake).toHaveBeenCalledTimes(1);

    // focus 后到（移动端获焦晚于可见）：应直接返回，不清复核定时器、不重跑恢复
    activity.handleFocus();
    await flush();
    expect(onWake).toHaveBeenCalledTimes(1);
    activity.cleanup();
  });

  it("bfcache 恢复强制重跑：pageshow persisted 无视边沿", async () => {
    stubDocument("visible", true);
    const activeSynced = ref<boolean | null>(null);
    const onWake = vi.fn(async () => {});
    const activity = useChatForegroundActivity({ activeSynced, onWake });

    await activity.sync("visibilitychange");
    await flush();
    expect(onWake).toHaveBeenCalledTimes(1);

    activity.handlePageShow({ persisted: true });
    await flush();
    expect(onWake).toHaveBeenCalledTimes(2);
    activity.cleanup();
  });

  it("唤醒失败回到未同步：延迟复核或下一次事件可重试", async () => {
    stubDocument("visible", false);
    const activeSynced = ref<boolean | null>(null);
    const onWake = vi.fn(async () => {});
    const onWakeError = vi.fn();
    restoreMock.mockRejectedValueOnce(new Error("network down"));
    const activity = useChatForegroundActivity({ activeSynced, onWake, onWakeError });

    await activity.sync("visibilitychange");
    await flush();

    expect(onWake).not.toHaveBeenCalled();
    expect(onWakeError).toHaveBeenCalledTimes(1);
    expect(activeSynced.value).toBeNull();

    await activity.sync("visibilitychange_recheck");
    await flush();
    expect(onWake).toHaveBeenCalledTimes(1);
    expect(activeSynced.value).toBe(true);
    activity.cleanup();
  });

  it("无冻结的 resume 不误唤醒：freeze 后 resume 才重跑", async () => {
    stubDocument("visible", true);
    const activeSynced = ref<boolean | null>(null);
    const onWake = vi.fn(async () => {});
    const activity = useChatForegroundActivity({ activeSynced, onWake });

    await activity.sync("visibilitychange");
    await flush();
    expect(onWake).toHaveBeenCalledTimes(1);

    activity.handleResume();
    await flush();
    expect(onWake).toHaveBeenCalledTimes(1);

    activity.handleFreeze();
    expect(activeSynced.value).toBe(false);
    activity.handleResume();
    await flush();
    expect(onWake).toHaveBeenCalledTimes(2);
    activity.cleanup();
  });
});
