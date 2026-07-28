import { decideForegroundRecovery } from "./foreground-recovery-decision";

/**
 * 焦点恢复的唯一状态机。
 *
 * 它只判定并恢复当前流投影：运行中绝不改写消息列表；只有确认已结束才交给宿主刷新
 * 目标消息或检查正式消息尾部。桌面、Web 和 IDE 宿主只能提供 I/O 适配，不能另写对账分支。
 */
export type ForegroundStreamIdentity = {
  activationId?: string;
  requestId?: string;
  updatedAt?: string;
  persistedAssistantMessageId?: string;
};

export type ForegroundRuntimeSnapshot = {
  runtimeState?: string;
  isProcessing?: boolean;
  hasPendingQueue?: boolean;
  pendingQueueCount?: number;
  streamCache?: ForegroundStreamIdentity | null;
};

export type ForegroundRecoveryInput = {
  conversationId: string;
  runtimeSnapshot: ForegroundRuntimeSnapshot;
  frontendStreaming: boolean;
  frontendMessageId?: string;
  frontendActivationId?: string;
  frontendRequestId?: string;
  frontendRevision?: string;
};

export type ForegroundRecoveryDependencies = {
  probeStream: (conversationId: string) => Promise<boolean>;
  resumeSubscription: (conversationId: string) => Promise<ForegroundRuntimeSnapshot | null>;
  applyRuntimeSnapshot: (runtimeSnapshot: ForegroundRuntimeSnapshot) => boolean | Promise<boolean>;
  refreshMessageById: (conversationId: string, messageId: string) => Promise<boolean>;
  finalizeMessage: (messageId: string) => void | Promise<void>;
};

export type ForegroundRecoveryOutcome = "handled" | "check_freshness" | "reload_conversation";

function normalized(value: unknown): string {
  return String(value || "").trim();
}

function runtimeIsActive(snapshot: ForegroundRuntimeSnapshot): boolean {
  const state = normalized(snapshot.runtimeState);
  return state === "assistant_streaming"
    || state === "organizing_context"
    || state === "compacting"
    || !!snapshot.isProcessing
    || !!snapshot.hasPendingQueue
    || Math.max(0, Number(snapshot.pendingQueueCount || 0)) > 0;
}

function targetMessageId(snapshot: ForegroundRuntimeSnapshot, fallbackMessageId?: string): string {
  return normalized(snapshot.streamCache?.persistedAssistantMessageId || fallbackMessageId);
}

function decide(
  input: ForegroundRecoveryInput,
  snapshot: ForegroundRuntimeSnapshot,
  probeState: "unknown" | "healthy" | "unhealthy",
) {
  return decideForegroundRecovery({
    backendStreaming: runtimeIsActive(snapshot),
    frontendStreaming: input.frontendStreaming,
    backendMessageId: snapshot.streamCache?.persistedAssistantMessageId,
    frontendMessageId: input.frontendMessageId,
    backendActivationId: snapshot.streamCache?.activationId,
    frontendActivationId: input.frontendActivationId,
    backendRequestId: snapshot.streamCache?.requestId,
    frontendRequestId: input.frontendRequestId,
    backendRevision: snapshot.streamCache?.updatedAt,
    frontendRevision: input.frontendRevision,
    probeState,
  });
}

export async function recoverForegroundStreaming(
  input: ForegroundRecoveryInput,
  dependencies: ForegroundRecoveryDependencies,
): Promise<ForegroundRecoveryOutcome> {
  let action = decide(input, input.runtimeSnapshot, "unknown");
  if (action === "probe_stream") {
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    action = decide(input, input.runtimeSnapshot, probeHealthy ? "healthy" : "unhealthy");
  }

  if (action === "keep") {
    return input.frontendStreaming || runtimeIsActive(input.runtimeSnapshot)
      ? "handled"
      : "check_freshness";
  }

  if (action === "refresh_target_message") {
    const messageId = targetMessageId(input.runtimeSnapshot, input.frontendMessageId);
    if (!messageId || !await dependencies.refreshMessageById(input.conversationId, messageId)) {
      return "reload_conversation";
    }
    await dependencies.finalizeMessage(messageId);
    return "handled";
  }

  if (action !== "resume_stream") return "reload_conversation";

  const resumedSnapshot = await dependencies.resumeSubscription(input.conversationId);
  const effectiveSnapshot = resumedSnapshot || input.runtimeSnapshot;
  const messageId = targetMessageId(effectiveSnapshot, input.frontendMessageId);
  if (runtimeIsActive(effectiveSnapshot)) {
    if (!messageId) return "reload_conversation";
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    if (!probeHealthy) return "reload_conversation";
    return await dependencies.applyRuntimeSnapshot(effectiveSnapshot)
      ? "handled"
      : "reload_conversation";
  }

  if (!messageId || !await dependencies.refreshMessageById(input.conversationId, messageId)) {
    return "reload_conversation";
  }
  await dependencies.finalizeMessage(messageId);
  return "handled";
}
