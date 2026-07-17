import { decideForegroundRecovery, type ForegroundRecoveryAction } from "./foreground-recovery-decision";

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

export type ForegroundSnapshotBindingStage =
  | "clear_runtime"
  | "unbind"
  | "request_snapshot"
  | "apply_snapshot"
  | "bind"
  | "resume";

export async function runForegroundSnapshotBindingTransaction<TSnapshot extends {
  shouldBindStream?: boolean;
  streamCache?: unknown;
}>(input: {
  conversationId: string;
  isCurrent: () => boolean;
  clearRuntime: () => void;
  unbind: () => Promise<void>;
  requestSnapshot: () => Promise<TSnapshot | null>;
  applySnapshot: (snapshot: TSnapshot) => void;
  bind: () => Promise<void>;
  resume: (snapshot: TSnapshot) => void;
  onStage?: (stage: ForegroundSnapshotBindingStage) => void;
  onUnbindError?: (error: unknown) => void;
}): Promise<TSnapshot | null> {
  input.onStage?.("clear_runtime");
  input.clearRuntime();
  input.onStage?.("unbind");
  const unbindPromise = input.unbind().catch((error) => {
    input.onUnbindError?.(error);
  });
  input.onStage?.("request_snapshot");
  const snapshot = await input.requestSnapshot();
  if (!snapshot || !input.isCurrent()) {
    await unbindPromise;
    return null;
  }
  input.onStage?.("apply_snapshot");
  input.applySnapshot(snapshot);
  await unbindPromise;
  if (!input.isCurrent()) return null;
  if (!snapshot.shouldBindStream) return snapshot;
  input.onStage?.("bind");
  await input.bind();
  if (!input.isCurrent()) return null;
  input.onStage?.("resume");
  input.resume(snapshot);
  return snapshot;
}

export async function reconcileForegroundConversation(input: {
  conversationId: string;
  isCurrent: () => boolean;
  requestRuntimeSnapshot: () => Promise<ForegroundRuntimeSnapshot>;
  applyRuntimeState: (snapshot: ForegroundRuntimeSnapshot) => void;
  frontendStreaming: () => boolean;
  readFrontendStreamCache: () => ForegroundStreamIdentity | null | undefined;
  probeStream: () => Promise<boolean>;
  readCurrentFormalTailMessageId: () => string;
  requestLatestFormalTailMessageId: () => Promise<string>;
  refreshTargetMessage: (messageId: string) => Promise<boolean>;
  resumeStream?: (snapshot: ForegroundRuntimeSnapshot) => Promise<boolean>;
  finalizeTargetRefresh: () => Promise<void> | void;
  reloadConversation: () => Promise<void>;
}): Promise<ForegroundRecoveryAction> {
  const snapshot = await input.requestRuntimeSnapshot();
  if (!input.isCurrent()) return "keep";
  input.applyRuntimeState(snapshot);
  const backendStreaming = snapshot.runtimeState === "assistant_streaming"
    || !!snapshot.isProcessing
    || !!snapshot.hasPendingQueue
    || Math.max(0, Number(snapshot.pendingQueueCount || 0)) > 0;
  const frontendStreaming = input.frontendStreaming();
  const frontendStreamCache = input.readFrontendStreamCache();
  const decisionInput = {
    backendStreaming,
    frontendStreaming,
    backendMessageId: snapshot.streamCache?.persistedAssistantMessageId,
    frontendMessageId: frontendStreamCache?.persistedAssistantMessageId,
    backendActivationId: snapshot.streamCache?.activationId,
    frontendActivationId: frontendStreamCache?.activationId,
    backendRequestId: snapshot.streamCache?.requestId,
    frontendRequestId: frontendStreamCache?.requestId,
    backendRevision: snapshot.streamCache?.updatedAt,
    frontendRevision: frontendStreamCache?.updatedAt,
  };
  let action = decideForegroundRecovery({ ...decisionInput, probeState: "unknown" });
  if (action === "probe_stream") {
    const healthy = await input.probeStream();
    if (!input.isCurrent()) return "keep";
    action = decideForegroundRecovery({
      ...decisionInput,
      probeState: healthy ? "healthy" : "unhealthy",
    });
  }
  if (action === "keep") {
    if (backendStreaming) return action;
    const latestTailMessageId = await input.requestLatestFormalTailMessageId();
    if (!input.isCurrent()) return "keep";
    const frontendTailMessageId = input.readCurrentFormalTailMessageId();
    if (latestTailMessageId === frontendTailMessageId) {
      if (!latestTailMessageId) return action;
      return await input.refreshTargetMessage(latestTailMessageId)
        ? "refresh_target_message"
        : action;
    }
    action = "reload_conversation";
  }
  if (action === "resume_stream" && input.resumeStream) {
    const resumed = await input.resumeStream(snapshot);
    if (!input.isCurrent()) return "keep";
    if (resumed) return action;
  }
  if (action === "refresh_target_message") {
    const messageId = String(
      snapshot.streamCache?.persistedAssistantMessageId
      || frontendStreamCache?.persistedAssistantMessageId
      || "",
    ).trim();
    if (messageId && await input.refreshTargetMessage(messageId)) {
      if (!input.isCurrent()) return "keep";
      await input.finalizeTargetRefresh();
      return action;
    }
  }
  await input.reloadConversation();
  return action === "refresh_target_message" ? "reload_conversation" : action;
}

export function createLatestTaskRunner<T>(task: (input: T) => Promise<void>) {
  let latestInput: T | undefined;
  let rerunRequested = false;
  let runningPromise: Promise<void> | null = null;
  let cancelled = false;

  function run(input: T): Promise<void> {
    latestInput = input;
    rerunRequested = true;
    if (runningPromise) return runningPromise;
    runningPromise = (async () => {
      while (rerunRequested && !cancelled) {
        rerunRequested = false;
        await task(latestInput as T);
      }
    })().finally(() => {
      runningPromise = null;
    });
    return runningPromise;
  }

  function cancel() {
    cancelled = true;
    rerunRequested = false;
  }

  return { run, cancel };
}
