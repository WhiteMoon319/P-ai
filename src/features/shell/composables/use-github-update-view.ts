import { computed, type Ref } from "vue";
import { useGithubUpdate } from "./use-github-update";
import type { GithubUpdateMethod } from "../../../types/app";

export type GithubUpdateViewBindings = {
  t: (key: string, params?: Record<string, unknown>) => string;
  viewMode: Ref<"chat" | "archives" | "config">;
  status: Ref<string>;
  updateMethod: Ref<GithubUpdateMethod | undefined>;
  skippedVersion: Ref<string | undefined>;
  onSkippedVersionSaved: (config: { skippedGithubUpdateVersion?: string }) => void;
};

export function useGithubUpdateView(bindings: GithubUpdateViewBindings) {
  const githubUpdate = useGithubUpdate({
    viewMode: bindings.viewMode,
    status: bindings.status,
    updateMethod: bindings.updateMethod,
    skippedVersion: bindings.skippedVersion,
    onSkippedVersionSaved: bindings.onSkippedVersionSaved,
  });

  const updateToLatestLabel = computed(() =>
    githubUpdate.updateReadyToRestart.value
      ? bindings.t("about.hasUpdate")
      : githubUpdate.updateInProgress.value
        ? bindings.t("about.updating")
        : bindings.t("about.hasUpdate"),
  );
  const updateToLatestTitle = computed(() => {
    const latestVersion = String(
      githubUpdate.currentUpdateState.value?.latestVersion || githubUpdate.latestCheckResult.value?.latestVersion || "",
    ).trim();
    if (githubUpdate.updateReadyToRestart.value && latestVersion) {
      return bindings.t("about.updateReadyAction", { version: latestVersion });
    }
    if (latestVersion) {
      return bindings.t("about.updateReadyAction", { version: latestVersion });
    }
    return bindings.t("about.hasUpdate");
  });

  return {
    ...githubUpdate,
    updateToLatestLabel,
    updateToLatestTitle,
  };
}
