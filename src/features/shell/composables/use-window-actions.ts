import { invokeTauri, openTransportExternalUrl, openTransportSettings, openTransportWindow } from "../../../services/tauri-api";

type UseWindowActionsOptions = {
  closeWindow: () => Promise<void>;
  minimizeWindow: () => Promise<void>;
  freezeForegroundConversation: (reason: string) => void;
};

export function useWindowActions(options: UseWindowActionsOptions) {
  function openSettingsWindow() {
    void openTransportSettings();
  }

  function summonChatWindowFromConfig() {
    options.freezeForegroundConversation("before_manual_summon");
    void openTransportWindow("chat");
  }

  async function closeWindowAndClearForeground() {
    options.freezeForegroundConversation("close_window");
    await options.closeWindow();
  }

  async function minimizeWindowAndClearForeground() {
    options.freezeForegroundConversation("minimize_window");
    await options.minimizeWindow();
  }

  async function openGithubRepository() {
    try {
      const url = await invokeTauri<string>("get_project_repository_url");
      void openTransportExternalUrl(url);
    } catch (error) {
      console.warn("[关于] 获取项目仓库地址失败:", error);
    }
  }

  return {
    openSettingsWindow,
    summonChatWindowFromConfig,
    closeWindowAndClearForeground,
    minimizeWindowAndClearForeground,
    openGithubRepository,
  };
}
