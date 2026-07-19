import { ref, type Ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import { toErrorMessage } from "../../../utils/error";
import type { ChatWorkspaceChoice } from "./use-chat-workspace";
import type { ShellWorkMode } from "../../../types/app";

type UseChatWorkspacePickerFlowOptions = {
  chatWorkspaceChoices: Ref<ChatWorkspaceChoice[]>;
  chatWorkspaceAutonomousMode: Ref<boolean>;
  chatWorkspaceWorkMode: Ref<ShellWorkMode>;
  openChatWorkspacePickerBase: () => void;
  closeChatWorkspacePickerBase: () => void;
  saveChatWorkspaces: (items: ChatWorkspaceChoice[], autonomousMode?: boolean, workMode?: ShellWorkMode) => Promise<void>;
  setStatus: (message: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  workspaceAlreadyExistsText: string;
  worktreeRequiresApprovalText: string;
  worktreeUnavailableText: string;
  checkChatWorkspaceGitRoot: (path: string) => Promise<boolean>;
};

export function useChatWorkspacePickerFlow(options: UseChatWorkspacePickerFlowOptions) {
  const chatWorkspaceDraftChoices = ref<ChatWorkspaceChoice[]>([]);
  const chatWorkspaceDraftAutonomousMode = ref(false);
  const chatWorkspaceDraftWorkMode = ref<ShellWorkMode>("directory");
  const chatWorkspaceDraftError = ref("");
  const chatWorkspacePickerSaving = ref(false);
  const tauriRuntimeAvailable = isTauriRuntimeAvailable();

  function cloneChatWorkspaceChoices(items: ChatWorkspaceChoice[]): ChatWorkspaceChoice[] {
    return (items || []).map((item) => ({
      id: String(item.id || "").trim(),
      name: String(item.name || "").trim(),
      path: String(item.path || "").trim(),
      level: item.level,
      access: item.access,
    }));
  }

  function syncChatWorkspaceDraftFromCurrentState() {
    chatWorkspaceDraftChoices.value = cloneChatWorkspaceChoices(options.chatWorkspaceChoices.value);
    chatWorkspaceDraftAutonomousMode.value = Boolean(options.chatWorkspaceAutonomousMode.value);
    chatWorkspaceDraftWorkMode.value = options.chatWorkspaceWorkMode.value;
    chatWorkspaceDraftError.value = "";
  }

  function openChatWorkspacePicker() {
    syncChatWorkspaceDraftFromCurrentState();
    options.openChatWorkspacePickerBase();
  }

  function closeChatWorkspacePicker() {
    if (chatWorkspacePickerSaving.value) return;
    options.closeChatWorkspacePickerBase();
    syncChatWorkspaceDraftFromCurrentState();
  }

  async function addChatWorkspace() {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
      });
      if (!picked || Array.isArray(picked)) return;
      const nextPath = String(picked || "").trim();
      if (!nextPath) return;
      const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
      const existed = draft.some((item) => String(item.path || "").trim().toLowerCase() === nextPath.toLowerCase());
      if (existed) {
        options.setStatus(options.workspaceAlreadyExistsText);
        return;
      }
      const hasMain = draft.some((item) => item.level === "main");
      draft.push({
        id: `conversation-workspace-${Math.random().toString(36).slice(2, 8)}`,
        name: nextPath.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || nextPath,
        path: nextPath,
        level: hasMain ? "secondary" : "main",
        access: hasMain ? "read_only" : "approval",
      });
      chatWorkspaceDraftChoices.value = draft;
    } catch (error) {
      options.setStatusError("status.requestFailed", error);
    }
  }

  async function setChatWorkspaceAsMain(workspaceId: string) {
    chatWorkspaceDraftError.value = "";
    const draft: ChatWorkspaceChoice[] = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value).map((item): ChatWorkspaceChoice => {
      if (item.level === "system") return item;
      if (item.id === workspaceId) {
        return { ...item, level: "main", access: item.access || "approval" };
      }
      if (item.level === "main") {
        return { ...item, level: "secondary" };
      }
      return item;
    });
    chatWorkspaceDraftChoices.value = draft;
  }

  function setChatWorkspaceAccess(workspaceId: string, access: ChatWorkspaceChoice["access"]) {
    chatWorkspaceDraftError.value = "";
    const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    const target = draft.find((item) => item.id === workspaceId);
    if (!target) return;
    if (target.level === "system") return;
    target.access = access;
    chatWorkspaceDraftChoices.value = draft;
  }

  async function removeChatWorkspace(workspaceId: string) {
    chatWorkspaceDraftError.value = "";
    const current = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    const removing = current.find((item) => item.id === workspaceId);
    const draft = current.filter((item) => item.id !== workspaceId || item.level === "system");
    if (removing?.level === "main") {
      const promoteTarget = draft.find((item) => item.level === "secondary");
      if (promoteTarget) {
        draft.forEach((item) => {
          if (item.level === "system") return;
          if (item.id === promoteTarget.id) {
            item.level = "main";
          } else if (item.level === "main") {
            item.level = "secondary";
          }
        });
      }
    }
    chatWorkspaceDraftChoices.value = draft;
  }

  function setChatWorkspaceAutonomousMode(enabled: boolean) {
    chatWorkspaceDraftAutonomousMode.value = Boolean(enabled);
  }

  function setChatWorkspaceWorkMode(mode: ShellWorkMode) {
    chatWorkspaceDraftError.value = "";
    chatWorkspaceDraftWorkMode.value = mode;
  }

  async function openChatWorkspaceDir(workspaceId: string) {
    if (!tauriRuntimeAvailable) return;
    const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
    const target = draft.find((item) => item.id === workspaceId);
    if (!target?.path) return;
    try {
      const opened = await invokeTauri<string>("open_chat_shell_workspace_dir", {
        input: { workspacePath: target.path },
      });
      options.setStatus(`已打开目录: ${opened}`);
    } catch (error) {
      options.setStatusError("config.tools.openDirFailed", error);
    }
  }

  async function saveChatWorkspacePicker() {
    if (chatWorkspacePickerSaving.value) return;
    chatWorkspacePickerSaving.value = true;
    try {
      const draft = cloneChatWorkspaceChoices(chatWorkspaceDraftChoices.value);
      const mainWorkspace = draft.find((item) => item.level === "main");
      if (chatWorkspaceDraftWorkMode.value === "isolated_worktree" && mainWorkspace?.access === "read_only") {
        chatWorkspaceDraftError.value = options.worktreeRequiresApprovalText;
        options.setStatus(options.worktreeRequiresApprovalText);
        return;
      }
      await options.saveChatWorkspaces(draft, chatWorkspaceDraftAutonomousMode.value, chatWorkspaceDraftWorkMode.value);
      options.closeChatWorkspacePickerBase();
      syncChatWorkspaceDraftFromCurrentState();
    } finally {
      chatWorkspacePickerSaving.value = false;
    }
  }

  return {
    chatWorkspaceDraftChoices,
    chatWorkspaceDraftAutonomousMode,
    chatWorkspaceDraftWorkMode,
    chatWorkspaceDraftError,
    chatWorkspacePickerSaving,
    openChatWorkspacePicker,
    closeChatWorkspacePicker,
    addChatWorkspace,
    setChatWorkspaceAsMain,
    setChatWorkspaceAccess,
    setChatWorkspaceAutonomousMode,
    setChatWorkspaceWorkMode,
    removeChatWorkspace,
    openChatWorkspaceDir,
    saveChatWorkspacePicker,
  };
}
