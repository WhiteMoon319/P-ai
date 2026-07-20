import type { Reactive } from "vue";
import type { AppConfig } from "../../../types/app";
import { invokeTauri } from "../../../services/tauri-api";

export function useGithubUpdateMethod(
  config: Reactive<AppConfig>,
  setStatusError: (i18nKey: string, error: unknown) => void,
) {
  function updateGithubUpdateMethod(value: unknown) {
    const nextMethod = value === "direct" || value === "proxy" ? value : "auto";
    if (config.githubUpdateMethod === nextMethod) return;
    const previousMethod = config.githubUpdateMethod || "auto";
    config.githubUpdateMethod = nextMethod;
    void invokeTauri<AppConfig>("set_github_update_method", { updateMethod: nextMethod })
      .then((saved) => {
        config.githubUpdateMethod = saved.githubUpdateMethod || "auto";
      })
      .catch((error) => {
        config.githubUpdateMethod = previousMethod;
        setStatusError("status.saveConfigFailed", error);
      });
  }

  return { updateGithubUpdateMethod };
}
