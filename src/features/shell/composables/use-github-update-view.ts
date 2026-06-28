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
      ? bindings.t("about.updateAndRestart")
      : githubUpdate.updateInProgress.value
        ? bindings.t("about.updating")
        : bindings.t("about.updateNow"),
  );
  const updateToLatestTitle = computed(() => {
    const latestVersion = String(githubUpdate.latestCheckResult.value?.latestVersion || "").trim();
    if (githubUpdate.updateReadyToRestart.value && latestVersion) {
      return bindings.t("about.updateReadyAction", { version: latestVersion });
    }
    if (latestVersion) {
      return bindings.t("about.updateAvailableAction", { version: latestVersion });
    }
    return bindings.t("about.updateNow");
  });

  return {
    ...githubUpdate,
    updateToLatestLabel,
    updateToLatestTitle,
  };
}
