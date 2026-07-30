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
  /** 后台整理/压缩没有可恢复的 assistant 正式消息；只更新忙态。 */
  applyBackgroundBusy?: (runtimeSnapshot: ForegroundRuntimeSnapshot) => void | Promise<void>;
};

export type ForegroundRecoveryOutcome = "handled" | "check_freshness" | "reload_conversation";

/**
 * 宿主无关的前台尾部对账入口。桌面、Web 和侧栏只能注入原子读取与 UI 适配，
 * 不得各自解释运行态后再写一份 freshness 回退。
 */
export async function reconcileForegroundRuntime(
  input: ForegroundRecoveryInput,
  dependencies: ForegroundRecoveryDependencies & {
    isCurrent: () => boolean;
    currentFormalTailMessageId: () => string;
    requestLatestFormalTailMessageId: (conversationId: string) => Promise<string>;
    reloadConversation: () => Promise<void>;
  },
): Promise<"handled" | "reloaded" | "stale"> {
  const outcome = await recoverForegroundStreaming(input, dependencies);
  if (!dependencies.isCurrent()) return "stale";
  if (outcome === "handled") return "handled";
  if (outcome === "reload_conversation") {
    await dependencies.reloadConversation();
    return dependencies.isCurrent() ? "reloaded" : "stale";
  }

  const currentTailId = dependencies.currentFormalTailMessageId();
  const latestTailId = await dependencies.requestLatestFormalTailMessageId(input.conversationId);
  if (!dependencies.isCurrent()) return "stale";
  if (latestTailId === currentTailId) return "handled";
  if (latestTailId && await dependencies.refreshMessageById(input.conversationId, latestTailId)) {
    return "handled";
  }
  await dependencies.reloadConversation();
  return dependencies.isCurrent() ? "reloaded" : "stale";
}

function normalized(value: unknown): string {
  return String(value || "").trim();
}

export type ForegroundRuntimeKind = "idle" | "assistant_streaming" | "background_busy";

export function classifyForegroundRuntime(snapshot: ForegroundRuntimeSnapshot): ForegroundRuntimeKind {
  const state = normalized(snapshot.runtimeState);
  if (state === "assistant_streaming" && !!normalized(snapshot.streamCache?.persistedAssistantMessageId)) {
    return "assistant_streaming";
  }
  if (state === "organizing_context"
    || state === "compacting"
    || !!snapshot.isProcessing
    || !!snapshot.hasPendingQueue
    || Math.max(0, Number(snapshot.pendingQueueCount || 0)) > 0) {
    return "background_busy";
  }
  return "idle";
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
    backendStreaming: classifyForegroundRuntime(snapshot) === "assistant_streaming",
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
  if (classifyForegroundRuntime(input.runtimeSnapshot) === "background_busy") {
    if (input.frontendMessageId) await dependencies.finalizeMessage(input.frontendMessageId);
    await dependencies.applyBackgroundBusy?.(input.runtimeSnapshot);
    return "handled";
  }
  let action = decide(input, input.runtimeSnapshot, "unknown");
  if (action === "probe_stream") {
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    action = decide(input, input.runtimeSnapshot, probeHealthy ? "healthy" : "unhealthy");
  }

  if (action === "keep") {
    return input.frontendStreaming || classifyForegroundRuntime(input.runtimeSnapshot) === "assistant_streaming"
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
  if (classifyForegroundRuntime(effectiveSnapshot) === "background_busy") {
    if (input.frontendMessageId) await dependencies.finalizeMessage(input.frontendMessageId);
    await dependencies.applyBackgroundBusy?.(effectiveSnapshot);
    return "handled";
  }
  if (classifyForegroundRuntime(effectiveSnapshot) === "assistant_streaming") {
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
