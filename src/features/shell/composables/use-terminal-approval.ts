import { computed, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";

export type TerminalApprovalRequestPayload = {
  requestId: string;
  title?: string;
  message?: string;
  approvalKind?: string;
  sessionId?: string;
  toolName?: string;
  summary?: string;
  callPreview?: string;
  cwd?: string;
  command?: string;
  requestedPath?: string;
  reason?: string;
  existingPaths?: string[];
  targetPaths?: string[];
  reviewOpinion?: string;
  reviewModelName?: string;
  canRememberWorkspace?: boolean;
  workspaceName?: string;
  workspacePath?: string;
};

export type TerminalApprovalConversationItem = TerminalApprovalRequestPayload & {
  conversationId: string;
};

type UseTerminalApprovalOptions = {
  queue: Ref<TerminalApprovalRequestPayload[]>;
  resolving: Ref<boolean>;
};

export function useTerminalApproval(options: UseTerminalApprovalOptions) {
  const terminalApprovalCurrent = computed(() => options.queue.value[0] ?? null);
  const terminalApprovalDialogOpen = computed(() => !!terminalApprovalCurrent.value);
  const terminalApprovalDialogTitle = computed(
    () => terminalApprovalCurrent.value?.title || "终端审批",
  );
  const terminalApprovalDialogBody = computed(
    () => terminalApprovalCurrent.value?.message || "",
  );

  function normalizeTerminalApprovalConversationId(payload: Pick<TerminalApprovalRequestPayload, "sessionId"> | null | undefined): string {
    const sessionId = String(payload?.sessionId || "").trim();
    if (!sessionId) return "";
    const parts = sessionId.split("::");
    if (parts.length >= 2) {
      return String(parts[parts.length - 1] || "").trim();
    }
    return sessionId;
  }

  function listConversationTerminalApprovals(conversationId: string): TerminalApprovalConversationItem[] {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return [];
    return options.queue.value
      .filter((item) => normalizeTerminalApprovalConversationId(item) === normalizedConversationId)
      .map((item) => ({
        ...item,
        conversationId: normalizedConversationId,
      }));
  }

  function getConversationTerminalApprovalCurrent(conversationId: string): TerminalApprovalConversationItem | null {
    return listConversationTerminalApprovals(conversationId)[0] ?? null;
  }

  function hasConversationTerminalApprovals(conversationId: string): boolean {
    return !!getConversationTerminalApprovalCurrent(conversationId);
  }

  function enqueueTerminalApprovalRequest(payload: TerminalApprovalRequestPayload) {
    const requestId = String(payload.requestId || "").trim();
    if (!requestId) return;
    if (options.queue.value.some((item) => item.requestId === requestId)) return;
    options.queue.value.push({
      ...payload,
      requestId,
      title: String(payload.title || "终端审批"),
      message: String(payload.message || ""),
      approvalKind: String(payload.approvalKind || "unknown"),
      sessionId: String(payload.sessionId || ""),
      toolName: String(payload.toolName || ""),
      summary: String(payload.summary || ""),
      callPreview: String(payload.callPreview || ""),
      cwd: String(payload.cwd || ""),
      command: String(payload.command || ""),
      requestedPath: String(payload.requestedPath || ""),
      reason: String(payload.reason || ""),
      reviewOpinion: String(payload.reviewOpinion || ""),
      reviewModelName: String(payload.reviewModelName || ""),
      canRememberWorkspace: !!payload.canRememberWorkspace,
      workspaceName: String(payload.workspaceName || ""),
      workspacePath: String(payload.workspacePath || ""),
      existingPaths: Array.isArray(payload.existingPaths)
        ? payload.existingPaths.map((item) => String(item || "").trim()).filter(Boolean)
        : [],
      targetPaths: Array.isArray(payload.targetPaths)
        ? payload.targetPaths.map((item) => String(item || "").trim()).filter(Boolean)
        : [],
    });
  }

  async function resolveTerminalApproval(approved: boolean, requestId?: string) {
    if (options.resolving.value) return;
    const normalizedRequestId = String(requestId || "").trim();
    const targetIndex = normalizedRequestId
      ? options.queue.value.findIndex((item) => item.requestId === normalizedRequestId)
      : 0;
    if (targetIndex < 0) return;
    const current = options.queue.value[targetIndex] ?? null;
    if (!current) return;
    options.resolving.value = true;
    try {
      await invokeTauri("resolve_terminal_approval", {
        input: {
          requestId: current.requestId,
          approved,
        },
      });
    } catch (error) {
      console.warn("[TERMINAL] resolve_terminal_approval failed:", error);
    } finally {
      options.queue.value.splice(targetIndex, 1);
      options.resolving.value = false;
    }
  }

  function denyTerminalApproval(requestId?: string) {
    void resolveTerminalApproval(false, requestId);
  }

  function approveTerminalApproval(requestId?: string) {
    void resolveTerminalApproval(true, requestId);
  }

  async function invokeTerminalApprovalAction(command: string, requestId?: string) {
    if (options.resolving.value) return;
    const normalizedRequestId = String(requestId || "").trim();
    const targetIndex = normalizedRequestId
      ? options.queue.value.findIndex((item) => item.requestId === normalizedRequestId)
      : 0;
    if (targetIndex < 0) return;
    const current = options.queue.value[targetIndex] ?? null;
    if (!current) return;
    options.resolving.value = true;
    try {
      await invokeTauri(command, {
        input: {
          requestId: current.requestId,
        },
      });
    } catch (error) {
      console.warn(`[TERMINAL] ${command} failed:`, error);
    } finally {
      options.queue.value.splice(targetIndex, 1);
      options.resolving.value = false;
    }
  }

  function approveTerminalApprovalForSession(requestId?: string) {
    void invokeTerminalApprovalAction("approve_terminal_approval_for_session", requestId);
  }

  function approveTerminalApprovalForWorkspace(requestId?: string) {
    void invokeTerminalApprovalAction("approve_terminal_approval_for_workspace", requestId);
  }

  return {
    terminalApprovalCurrent,
    terminalApprovalDialogOpen,
    terminalApprovalDialogTitle,
    terminalApprovalDialogBody,
    listConversationTerminalApprovals,
    getConversationTerminalApprovalCurrent,
    hasConversationTerminalApprovals,
    enqueueTerminalApprovalRequest,
    denyTerminalApproval,
    approveTerminalApproval,
    approveTerminalApprovalForSession,
    approveTerminalApprovalForWorkspace,
  };
}
