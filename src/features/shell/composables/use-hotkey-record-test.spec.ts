import { afterEach, describe, expect, it, vi } from "vitest";
import { useHotkeyRecordTest } from "./use-hotkey-record-test";

function stubMicrophoneEnvironment(input: {
  getUserMedia: ReturnType<typeof vi.fn>;
  permissionState?: PermissionState;
  userAgent?: string;
}) {
  const permissionStatus = {
    state: input.permissionState ?? "prompt",
    onchange: null,
  } as unknown as PermissionStatus;
  const permissionQuery = vi.fn(async () => permissionStatus);

  vi.stubGlobal("navigator", {
    userAgent: input.userAgent || "",
    mediaDevices: { getUserMedia: input.getUserMedia },
    permissions: { query: permissionQuery },
  });
  vi.stubGlobal("MediaRecorder", class {});

  return { permissionQuery };
}

function createRecordTest() {
  return useHotkeyRecordTest({
    t: (key) => key,
    setStatus: vi.fn(),
    setStatusError: vi.fn(),
  });
}

function microphonePermissionError(name = "NotAllowedError") {
  return Object.assign(new Error("Permission denied"), { name });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("useHotkeyRecordTest microphone permission", () => {
  it("keeps granted after getUserMedia succeeds when Permissions API still reports prompt", async () => {
    const stop = vi.fn();
    const getUserMedia = vi.fn(async () => ({
      getTracks: () => [{ stop }],
    }));
    const { permissionQuery } = stubMicrophoneEnvironment({ getUserMedia });
    const recordTest = createRecordTest();
    await Promise.resolve();

    expect(recordTest.microphonePermissionState.value).toBe("prompt");
    await expect(recordTest.requestMicrophonePermission()).resolves.toBe(true);

    expect(recordTest.microphonePermissionState.value).toBe("granted");
    expect(permissionQuery).toHaveBeenCalledTimes(1);
    expect(stop).toHaveBeenCalledOnce();
  });

  it("keeps denied after getUserMedia rejects with a permission error", async () => {
    const getUserMedia = vi.fn(async () => {
      throw microphonePermissionError();
    });
    const { permissionQuery } = stubMicrophoneEnvironment({ getUserMedia });
    const recordTest = createRecordTest();
    await Promise.resolve();

    expect(recordTest.microphonePermissionState.value).toBe("prompt");
    await expect(recordTest.requestMicrophonePermission()).resolves.toBe(false);

    expect(recordTest.microphonePermissionState.value).toBe("denied");
    expect(permissionQuery).toHaveBeenCalledTimes(1);
  });

  it("keeps unknown after getUserMedia rejects with a non-permission capture error", async () => {
    const getUserMedia = vi.fn(async () => {
      throw Object.assign(new Error("Could not start audio source"), { name: "NotReadableError" });
    });
    stubMicrophoneEnvironment({ getUserMedia });
    const recordTest = createRecordTest();
    await Promise.resolve();

    expect(recordTest.microphonePermissionState.value).toBe("prompt");
    await expect(recordTest.requestMicrophonePermission()).resolves.toBe(false);

    expect(recordTest.microphonePermissionState.value).toBe("unknown");
  });

  it("does not trust Permissions API state on Android WebView", async () => {
    const stop = vi.fn();
    const getUserMedia = vi.fn(async () => ({
      getTracks: () => [{ stop }],
    }));
    const { permissionQuery } = stubMicrophoneEnvironment({ getUserMedia, permissionState: "denied" });
    vi.stubGlobal("window", { location: { search: "?platform=android" } });
    const recordTest = createRecordTest();
    await Promise.resolve();

    expect(recordTest.microphonePermissionState.value).toBe("unknown");
    expect(permissionQuery).not.toHaveBeenCalled();

    await expect(recordTest.requestMicrophonePermission()).resolves.toBe(true);
    expect(recordTest.microphonePermissionState.value).toBe("granted");
    expect(stop).toHaveBeenCalledOnce();
  });
});
