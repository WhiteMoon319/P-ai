import { ref } from "vue";
import { describe, expect, it, vi } from "vitest";
import type { ShellWorkMode } from "../../../types/app";
import { useChatWorkspacePickerFlow } from "./use-chat-workspace-picker-flow";

function createFlow(access: "read_only" | "approval") {
  const saveChatWorkspaces = vi.fn(async () => undefined);
  const setStatus = vi.fn();
  const flow = useChatWorkspacePickerFlow({
    chatWorkspaceChoices: ref([{
      id: "main-workspace",
      name: "项目",
      path: "E:/project",
      level: "main",
      access,
    }]),
    chatWorkspaceAutonomousMode: ref(false),
    chatWorkspaceWorkMode: ref<ShellWorkMode>("isolated_worktree"),
    openChatWorkspacePickerBase: vi.fn(),
    closeChatWorkspacePickerBase: vi.fn(),
    saveChatWorkspaces,
    setStatus,
    setStatusError: vi.fn(),
    workspaceAlreadyExistsText: "目录已存在",
    worktreeRequiresApprovalText: "在隔离工作树中工作至少需要审批权限。",
    worktreeUnavailableText: "目录不是 Git 根目录",
    checkChatWorkspaceGitRoot: vi.fn(async () => true),
  });
  flow.openChatWorkspacePicker();
  return { flow, saveChatWorkspaces, setStatus };
}

describe("useChatWorkspacePickerFlow", () => {
  it("blocks saving isolated worktree mode for a read-only shell workspace", async () => {
    const { flow, saveChatWorkspaces, setStatus } = createFlow("read_only");

    await flow.saveChatWorkspacePicker();

    expect(saveChatWorkspaces).not.toHaveBeenCalled();
    expect(setStatus).toHaveBeenCalledWith("在隔离工作树中工作至少需要审批权限。");
    expect(flow.chatWorkspaceDraftError.value).toBe("在隔离工作树中工作至少需要审批权限。");
  });

  it("persists isolated worktree mode when shell workspace access is approval", async () => {
    const { flow, saveChatWorkspaces } = createFlow("approval");

    await flow.saveChatWorkspacePicker();

    expect(saveChatWorkspaces).toHaveBeenCalledWith(
      expect.any(Array),
      false,
      "isolated_worktree",
    );
  });
});
