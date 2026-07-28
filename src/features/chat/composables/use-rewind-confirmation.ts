import { ref, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";

export type RecallMode = "with_patch" | "message_only" | "cancel";

type RewindConversationPreviewResult = {
  conversationId: string;
  canUndoPatch: boolean;
  hint: string;
};

type UseRewindConfirmationOptions = {
  currentConversationId: Ref<string>;
};

/** 撤回与从消息建分支的确认状态只保留一份，供所有聊天宿主复用。 */
export function useRewindConfirmation(options: UseRewindConfirmationOptions) {
  const rewindConfirmDialogOpen = ref(false);
  const rewindConfirmCanUndoPatch = ref(false);
  const rewindConfirmUndoHint = ref("");
  let rewindConfirmResolver: ((mode: RecallMode) => void) | null = null;
  const branchFromMessageConfirmDialogOpen = ref(false);
  let branchFromMessageConfirmResolver: ((confirmed: boolean) => void) | null = null;

  async function getUndoAvailabilityForTurn(targetMessageId: string): Promise<{ canUndo: boolean; hint: string }> {
    const conversationId = String(options.currentConversationId.value || "").trim();
    const messageId = String(targetMessageId || "").trim();
    if (!messageId || !conversationId) {
      return { canUndo: false, hint: "缺少撤回预览所需的会话上下文。" };
    }
    try {
      const preview = await invokeTauri<RewindConversationPreviewResult>("conversation.rewindPreview", {
        session: { agentId: "", conversationId },
        messageId,
        undoApplyPatch: false,
      });
      return {
        canUndo: !!preview.canUndoPatch,
        hint: String(preview.hint || "").trim(),
      };
    } catch (error) {
      console.warn("[会话撤回] 撤回预览失败，隐藏文件修改撤回入口", {
        messageId,
        conversationId,
        error,
      });
      return { canUndo: false, hint: "撤回预览失败，仅撤回消息。" };
    }
  }

  async function requestRecallMode(payload: { turnId: string; targetUserMessageId: string }): Promise<RecallMode> {
    cancelPendingRewindConfirm();
    const availability = await getUndoAvailabilityForTurn(payload.targetUserMessageId);
    rewindConfirmCanUndoPatch.value = availability.canUndo;
    rewindConfirmUndoHint.value = availability.hint;
    rewindConfirmDialogOpen.value = true;
    return new Promise((resolve) => {
      rewindConfirmResolver = resolve;
    });
  }

  function resolveRewindConfirm(mode: RecallMode) {
    const resolver = rewindConfirmResolver;
    rewindConfirmResolver = null;
    rewindConfirmDialogOpen.value = false;
    rewindConfirmCanUndoPatch.value = false;
    rewindConfirmUndoHint.value = "";
    resolver?.(mode);
  }

  function confirmRewindWithPatch() {
    resolveRewindConfirm("with_patch");
  }

  function confirmRewindMessageOnly() {
    resolveRewindConfirm("message_only");
  }

  function cancelRewindConfirm() {
    resolveRewindConfirm("cancel");
  }

  function cancelPendingRewindConfirm() {
    if (!rewindConfirmResolver) {
      rewindConfirmDialogOpen.value = false;
      rewindConfirmCanUndoPatch.value = false;
      rewindConfirmUndoHint.value = "";
      return;
    }
    const resolver = rewindConfirmResolver;
    rewindConfirmResolver = null;
    rewindConfirmDialogOpen.value = false;
    rewindConfirmCanUndoPatch.value = false;
    rewindConfirmUndoHint.value = "";
    resolver("cancel");
  }

  function requestCreateConversationBranchFromMessageConfirm(): Promise<boolean> {
    cancelPendingBranchFromMessageConfirm();
    branchFromMessageConfirmDialogOpen.value = true;
    return new Promise((resolve) => {
      branchFromMessageConfirmResolver = resolve;
    });
  }

  function resolveBranchFromMessageConfirm(confirmed: boolean) {
    const resolver = branchFromMessageConfirmResolver;
    branchFromMessageConfirmResolver = null;
    branchFromMessageConfirmDialogOpen.value = false;
    resolver?.(confirmed);
  }

  function confirmBranchFromMessage() {
    resolveBranchFromMessageConfirm(true);
  }

  function cancelBranchFromMessageConfirm() {
    resolveBranchFromMessageConfirm(false);
  }

  function cancelPendingBranchFromMessageConfirm() {
    if (!branchFromMessageConfirmResolver) {
      branchFromMessageConfirmDialogOpen.value = false;
      return;
    }
    resolveBranchFromMessageConfirm(false);
  }

  return {
    rewindConfirmDialogOpen,
    rewindConfirmCanUndoPatch,
    rewindConfirmUndoHint,
    branchFromMessageConfirmDialogOpen,
    requestRecallMode,
    confirmRewindWithPatch,
    confirmRewindMessageOnly,
    cancelRewindConfirm,
    cancelPendingRewindConfirm,
    requestCreateConversationBranchFromMessageConfirm,
    confirmBranchFromMessage,
    cancelBranchFromMessageConfirm,
    cancelPendingBranchFromMessageConfirm,
  };
}
