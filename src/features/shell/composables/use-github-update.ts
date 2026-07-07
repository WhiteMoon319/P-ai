import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onBeforeUnmount, ref, type Ref } from "vue";
import { i18n } from "../../../i18n";
import { invokeTauri, isTauriRuntimeAvailable, onWebBridgeNotification } from "../../../services/tauri-api";
import type { GithubUpdateInfo, UpdateProgressPayload } from "../types/update";
import type { GithubUpdateMethod } from "../../../types/app";

const t = i18n.global.t;

type ViewModeRef = Ref<"chat" | "archives" | "config">;

type UseGithubUpdateOptions = {
  viewMode: ViewModeRef;
  status: Ref<string>;
  updateMethod: Ref<GithubUpdateMethod | undefined>;
  skippedVersion: Ref<string | undefined>;
  onSkippedVersionSaved: (config: { skippedGithubUpdateVersion?: string }) => void;
};

function formatBytes(value?: number) {
  if (!Number.isFinite(value) || !value || value <= 0) return "";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let idx = 0;
  while (size >= 1024 && idx < units.length - 1) {
    size /= 1024;
    idx += 1;
  }
  const digits = idx === 0 ? 0 : size >= 100 ? 0 : size >= 10 ? 1 : 2;
  return `${size.toFixed(digits)} ${units[idx]}`;
}

function normalizeSkippedVersion(value: string | undefined) {
  return String(value || "").trim();
}

function isCancellableUpdateStage(stage: string | null | undefined) {
  return ["checking", "downloading", "verifying", "preparing", "replacing"].includes(String(stage || ""));
}

export function useGithubUpdate(options: UseGithubUpdateOptions) {
  const checkingUpdateRequest = ref(false);
  const updateInProgress = ref(false);
  const updateCancelPending = ref(false);
  const updateStage = ref<string | null>(null);
  const updateReadyToRestart = ref(false);
  const updateDialogOpen = ref(false);
  const updateDialogTitle = ref(t("about.dialogTitleCheck"));
  const updateDialogBody = ref("");
  const updateDialogKind = ref<"info" | "error">("info");
  const updateDialogReleaseUrl = ref("");
  const updateDialogPrimaryAction = ref<"download" | "force" | "restart" | null>(null);
  const updateProgressPercent = ref<number | null>(null);
  const updateRuntimeKind = ref<"installer" | "portable">("installer");
  const latestCheckResult = ref<GithubUpdateInfo | null>(null);
  const checkingUpdate = computed(() => checkingUpdateRequest.value || updateInProgress.value);
  const updateUiMode = ref<"foreground" | "background" | null>(null);
  const skippedVersion = computed(() => normalizeSkippedVersion(options.skippedVersion.value));

  const updateSuppressedBySkip = computed(() => {
    const latestVersion = String(latestCheckResult.value?.latestVersion || "").trim();
    return !!latestVersion && !!skippedVersion.value && latestVersion === skippedVersion.value;
  });
  const shouldShowUpdateAction = computed(() => {
    if (updateReadyToRestart.value || updateInProgress.value) return true;
    return !!latestCheckResult.value?.hasUpdate && !updateSuppressedBySkip.value;
  });
  const hasAvailableUpdate = computed(() => shouldShowUpdateAction.value);
  const showUpdateToLatestButton = computed(() => shouldShowUpdateAction.value);
  const latestUpdateVersion = computed(() => String(latestCheckResult.value?.latestVersion || "").trim());
  const updateDialogSkipVersionVisible = computed(() =>
    updateDialogOpen.value
    && updateDialogPrimaryAction.value === "download"
    && !!latestUpdateVersion.value
    && !updateInProgress.value
    && !updateReadyToRestart.value,
  );
  const updateDialogCancelUpdateVisible = computed(() =>
    updateInProgress.value
    && !updateCancelPending.value
    && isCancellableUpdateStage(updateStage.value),
  );

  let updateProgressUnlisten: UnlistenFn | null = null;
  let webUpdateProgressUnlisten: (() => void) | null = null;
  let autoCheckTimer: number | null = null;
  let autoCheckStarted = false;
  const AUTO_CHECK_INTERVAL_MS = 8 * 60 * 60 * 1000;

  function runtimeLabel(kind: "installer" | "portable") {
    return kind === "portable" ? t("about.runtimePortable") : t("about.runtimeInstaller");
  }

  function closeUpdateDialog() {
    updateDialogOpen.value = false;
  }

  function openUpdateDialog(text: string, kind: "info" | "error", releaseUrl?: string) {
    updateDialogTitle.value = t("about.dialogTitleCheck");
    updateDialogBody.value = text;
    updateDialogKind.value = kind;
    updateDialogReleaseUrl.value = releaseUrl || "";
    updateDialogPrimaryAction.value = null;
    updateProgressPercent.value = null;
    updateStage.value = null;
    updateDialogOpen.value = true;
  }

  function openUpdateRelease() {
    const url = String(updateDialogReleaseUrl.value || latestCheckResult.value?.releaseUrl || "").trim();
    if (!url) return;
    void invokeTauri("open_external_url", { url });
  }

  function buildCheckDialogBody(result: GithubUpdateInfo) {
    const lines = [
      t("about.currentVersion", { version: result.currentVersion }),
      t("about.latestVersion", { version: result.latestVersion }),
      t("about.currentRuntime", { kind: runtimeLabel(result.runtimeKind) }),
    ];
    const notes = String(result.releaseNotes || "").trim();
    if (notes) {
      lines.push("");
      lines.push(t("about.releaseNotes"));
      lines.push(notes);
    }
    return lines.join("\n");
  }

  function openCheckResultDialog(result: GithubUpdateInfo) {
    updateDialogReleaseUrl.value = result.releaseUrl || "";
    updateDialogBody.value = buildCheckDialogBody(result);
    updateDialogKind.value = "info";
    updateDialogPrimaryAction.value = result.hasUpdate ? "download" : "force";
    updateProgressPercent.value = null;
    updateStage.value = null;
    updateDialogTitle.value = result.hasUpdate ? t("about.foundUpdate") : t("about.alreadyLatest");
    updateDialogOpen.value = true;
  }

  function clearAutoCheckTimer() {
    if (autoCheckTimer != null) {
      window.clearTimeout(autoCheckTimer);
      autoCheckTimer = null;
    }
  }

  function scheduleNextAutoCheck() {
    clearAutoCheckTimer();
    autoCheckTimer = window.setTimeout(() => {
      void checkGithubUpdate(true, true).finally(() => {
        scheduleNextAutoCheck();
      });
    }, AUTO_CHECK_INTERVAL_MS);
  }

  function currentUpdateMethod(): GithubUpdateMethod {
    const value = options.updateMethod.value;
    return value === "direct" || value === "proxy" ? value : "auto";
  }

  function applySkippedVersion(version: string) {
    options.onSkippedVersionSaved({
      skippedGithubUpdateVersion: String(version || "").trim(),
    });
  }

  async function saveSkippedVersion(version: string) {
    const saved = await invokeTauri<{ skippedGithubUpdateVersion?: string }>("set_skipped_github_update_version", { version });
    applySkippedVersion(saved.skippedGithubUpdateVersion || "");
  }

  function syncDialogFromProgress(payload: UpdateProgressPayload) {
    const previousUiMode = updateUiMode.value;
    updateStage.value = payload.stage;
    updateRuntimeKind.value = payload.runtimeKind;
    updateDialogReleaseUrl.value = latestCheckResult.value?.releaseUrl || "";
    updateProgressPercent.value = Number.isFinite(payload.percent) ? payload.percent ?? null : null;
    if (payload.stage === "failed") {
      updateInProgress.value = false;
      updateCancelPending.value = false;
      updateReadyToRestart.value = false;
      updateUiMode.value = null;
      updateDialogPrimaryAction.value = null;
      updateDialogKind.value = "error";
      updateDialogTitle.value = t("about.updateFailed");
      updateDialogBody.value = payload.error ? `${payload.message}\n\n${payload.error}` : payload.message;
      if (previousUiMode !== "background") {
        updateDialogOpen.value = true;
      }
      return;
    }
    if (payload.stage === "cancelled") {
      updateInProgress.value = false;
      updateCancelPending.value = false;
      updateReadyToRestart.value = false;
      updateUiMode.value = null;
      updateDialogPrimaryAction.value = null;
      updateProgressPercent.value = null;
      updateDialogOpen.value = false;
      return;
    }
    if (payload.stage === "ready") {
      updateInProgress.value = false;
      updateCancelPending.value = false;
      updateReadyToRestart.value = true;
      updateUiMode.value = null;
      if (latestCheckResult.value) {
        latestCheckResult.value = {
          ...latestCheckResult.value,
          hasUpdate: true,
        };
      }
      if (previousUiMode !== "background") {
        updateDialogOpen.value = true;
      }
      updateDialogKind.value = "info";
      updateDialogTitle.value = t("about.updateDownloaded");
      updateDialogBody.value = payload.message;
      updateDialogPrimaryAction.value = "restart";
      updateProgressPercent.value = 100;
      return;
    }
    updateDialogKind.value = "info";
    updateDialogTitle.value = payload.stage === "completed" ? t("about.updateCompleted") : t("about.downloading");
    const progressLine =
      Number.isFinite(payload.downloadedBytes) || Number.isFinite(payload.contentLength)
        ? `\n\n${t("about.downloadProgress", { current: formatBytes(payload.downloadedBytes), total: formatBytes(payload.contentLength) })}${
            Number.isFinite(payload.percent) ? ` (${Math.max(0, Math.min(100, payload.percent || 0)).toFixed(1)}%)` : ""
          }`
        : "";
    updateDialogBody.value = `${payload.message}\n\n${t("about.currentRuntime", { kind: runtimeLabel(payload.runtimeKind) })}${progressLine}`;
    if (payload.stage === "completed") {
      updateInProgress.value = false;
      updateCancelPending.value = false;
      updateReadyToRestart.value = false;
      updateUiMode.value = null;
      updateDialogOpen.value = true;
      updateDialogPrimaryAction.value = null;
      return;
    }
    if (previousUiMode !== "background") {
      updateDialogOpen.value = true;
      updateDialogPrimaryAction.value = null;
    }
  }

  function isUpdateProgressPayload(payload: unknown): payload is UpdateProgressPayload {
    return !!payload && typeof payload === "object" && typeof (payload as UpdateProgressPayload).stage === "string";
  }

  function handleUpdateProgressPayload(payload: UpdateProgressPayload | null | undefined) {
    if (!payload) return;
    updateInProgress.value = !["failed", "completed", "ready", "cancelled"].includes(payload.stage);
    syncDialogFromProgress(payload);
    options.status.value = payload.error ? payload.error : payload.message;
  }

  async function checkGithubUpdate(silent: boolean, useCachedResult = false) {
    if (options.viewMode.value === "archives") return;
    if (checkingUpdate.value) return;
    checkingUpdateRequest.value = true;
    try {
      if (!silent) {
        options.status.value = t("about.checking");
      }
      const result = await invokeTauri<GithubUpdateInfo>("check_github_update", {
        updateMethod: currentUpdateMethod(),
        useCachedResult,
      });
      latestCheckResult.value = result;
      updateRuntimeKind.value = result.runtimeKind;
      updateDialogReleaseUrl.value = result.releaseUrl || "";
      if (!result?.hasUpdate) {
        updateReadyToRestart.value = false;
        if (!silent) {
          options.status.value = t("about.alreadyLatestWithVersion", { version: result.currentVersion });
          openCheckResultDialog(result);
        }
        return result;
      }
      options.status.value = t("about.foundNewVersion", { latest: result.latestVersion, current: result.currentVersion });
      if (!silent || !updateSuppressedBySkip.value) {
        openCheckResultDialog(result);
      }
      return result;
    } catch (error) {
      if (!silent) {
        options.status.value = t("about.checkFailed", { error: String(error) });
        updateDialogPrimaryAction.value = null;
        openUpdateDialog(t("about.checkFailedDialog", { error: String(error) }), "error");
      }
      console.warn("[UPDATE] check_github_update failed:", error);
    } finally {
      checkingUpdateRequest.value = false;
    }
  }

  async function startGithubUpdate(force: boolean, silent: boolean) {
    if (checkingUpdate.value) return;
    updateInProgress.value = true;
    updateCancelPending.value = false;
    updateStage.value = "checking";
    updateReadyToRestart.value = false;
    updateUiMode.value = silent ? "background" : "foreground";
    updateDialogPrimaryAction.value = null;
    updateDialogKind.value = "info";
    updateDialogTitle.value = force ? t("about.prepareForceDownload") : t("about.prepareDownload");
    updateDialogBody.value = force ? t("about.preparingForceDownload") : t("about.preparingDownload");
    updateProgressPercent.value = null;
    options.status.value = force ? t("about.preparingForceDownload") : t("about.preparingDownload");
    if (!silent) {
      updateDialogOpen.value = true;
    }
    try {
      await invokeTauri("start_github_update", { force, updateMethod: currentUpdateMethod() });
    } catch (error) {
      if (String(error || "").includes("用户已取消更新")) {
        updateInProgress.value = false;
        updateCancelPending.value = false;
        updateStage.value = null;
        updateUiMode.value = null;
        updateDialogOpen.value = false;
        updateProgressPercent.value = null;
        options.status.value = t("about.cancellingUpdate");
        return;
      }
      updateInProgress.value = false;
      updateCancelPending.value = false;
      updateStage.value = null;
      updateUiMode.value = null;
      updateDialogKind.value = "error";
      updateDialogTitle.value = t("about.updateFailed");
      updateDialogBody.value = t("about.startUpdateFailed", { error: String(error) });
      if (!silent) {
        updateDialogOpen.value = true;
      }
      options.status.value = t("about.startUpdateFailedStatus", { error: String(error) });
      console.warn("[UPDATE] start_github_update failed:", error);
    }
  }

  async function cancelGithubUpdate() {
    if (!updateInProgress.value || updateCancelPending.value || !isCancellableUpdateStage(updateStage.value)) return;
    updateCancelPending.value = true;
    options.status.value = t("about.cancellingUpdate");
    try {
      await invokeTauri("cancel_github_update");
    } catch (error) {
      updateCancelPending.value = false;
      options.status.value = t("about.cancelUpdateFailed", { error: String(error) });
      openUpdateDialog(t("about.cancelUpdateFailed", { error: String(error) }), "error");
    }
  }

  async function applyPreparedGithubUpdate() {
    if (checkingUpdate.value) return;
    updateInProgress.value = true;
    updateCancelPending.value = false;
    updateStage.value = "installing";
    updateUiMode.value = "foreground";
    updateDialogOpen.value = true;
    updateDialogKind.value = "info";
    updateDialogPrimaryAction.value = null;
    updateDialogTitle.value = t("about.updateAndRestartTitle");
    updateDialogBody.value = t("about.applyingUpdate");
    updateProgressPercent.value = null;
    options.status.value = t("about.applyingUpdate");
    try {
      await invokeTauri("apply_prepared_github_update");
    } catch (error) {
      updateInProgress.value = false;
      updateCancelPending.value = false;
      updateStage.value = null;
      updateUiMode.value = null;
      updateDialogKind.value = "error";
      updateDialogTitle.value = t("about.updateFailed");
      updateDialogBody.value = t("about.applyUpdateFailed", { error: String(error) });
      options.status.value = t("about.applyUpdateFailedStatus", { error: String(error) });
      console.warn("[UPDATE] apply_prepared_github_update failed:", error);
    }
  }

  function confirmUpdateDialogPrimary() {
    if (updateDialogPrimaryAction.value === "download") {
      void startGithubUpdate(false, false);
      return;
    }
    if (updateDialogPrimaryAction.value === "force") {
      void startGithubUpdate(true, false);
      return;
    }
    if (updateDialogPrimaryAction.value === "restart") {
      void applyPreparedGithubUpdate();
    }
  }

  async function skipCurrentUpdateVersion() {
    const version = latestUpdateVersion.value;
    if (!version) return;
    await saveSkippedVersion(version);
    updateDialogOpen.value = false;
    options.status.value = t("about.skipVersionSaved", { version });
  }

  async function autoCheckGithubUpdate() {
    if (autoCheckStarted) return;
    autoCheckStarted = true;
    await checkGithubUpdate(true, true);
    scheduleNextAutoCheck();
  }

  async function manualCheckGithubUpdate() {
    await checkGithubUpdate(false);
  }

  async function triggerUpdateToLatest() {
    if (updateReadyToRestart.value) {
      await applyPreparedGithubUpdate();
      return;
    }
    if (updateInProgress.value || checkingUpdateRequest.value) {
      if (updateUiMode.value !== "background") {
        updateDialogOpen.value = true;
      }
      return;
    }
    if (latestCheckResult.value?.hasUpdate) {
      if (updateSuppressedBySkip.value) {
        await saveSkippedVersion("");
      }
      await startGithubUpdate(false, false);
      return;
    }
    const result = await checkGithubUpdate(false);
    if (result?.hasUpdate) {
      if (updateSuppressedBySkip.value) {
        await saveSkippedVersion("");
      }
    }
  }

  if (isTauriRuntimeAvailable()) {
    void listen<UpdateProgressPayload>("easy-call:update-status", (event) => {
      handleUpdateProgressPayload(event.payload);
    })
      .then((unlisten) => {
        updateProgressUnlisten = unlisten;
      })
      .catch((error) => {
        console.warn("[UPDATE] listen easy-call:update-status failed:", error);
      });
  } else {
    webUpdateProgressUnlisten = onWebBridgeNotification("easy-call:update-status", (payload) => {
      if (isUpdateProgressPayload(payload)) {
        handleUpdateProgressPayload(payload);
      }
    });
  }

  onBeforeUnmount(() => {
    updateProgressUnlisten?.();
    updateProgressUnlisten = null;
    webUpdateProgressUnlisten?.();
    webUpdateProgressUnlisten = null;
    clearAutoCheckTimer();
    autoCheckStarted = false;
  });

  return {
    checkingUpdate,
    hasAvailableUpdate,
    updateReadyToRestart,
    updateInProgress,
    updateCancelPending,
    latestCheckResult,
    updateDialogOpen,
    updateDialogTitle,
    updateDialogBody,
    updateDialogKind,
    updateDialogReleaseUrl,
    updateDialogPrimaryAction,
    updateProgressPercent,
    updateDialogSkipVersionVisible,
    updateDialogCancelUpdateVisible,
    closeUpdateDialog,
    openUpdateRelease,
    confirmUpdateDialogPrimary,
    autoCheckGithubUpdate,
    manualCheckGithubUpdate,
    triggerUpdateToLatest,
    cancelGithubUpdate,
    skipCurrentUpdateVersion,
    showUpdateToLatestButton,
  };
}
