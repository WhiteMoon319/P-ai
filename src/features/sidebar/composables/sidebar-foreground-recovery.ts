import { decideForegroundRecovery } from "../../chat/composables/foreground-recovery-decision";

export type SidebarForegroundRuntimeSnapshot = {
  runtimeState?: string;
  streamCache?: {
    activationId?: string;
    requestId?: string;
    updatedAt?: string;
    persistedAssistantMessageId?: string;
  } | null;
};

export type SidebarForegroundRecoveryInput = {
  conversationId: string;
  runtimeSnapshot: SidebarForegroundRuntimeSnapshot;
  frontendStreaming: boolean;
  frontendMessageId?: string;
  frontendActivationId?: string;
  frontendRequestId?: string;
  frontendRevision?: string;
};

export type SidebarForegroundRecoveryDependencies = {
  probeStream: (conversationId: string) => Promise<boolean>;
  resumeSubscription: (conversationId: string) => Promise<SidebarForegroundRuntimeSnapshot | null>;
  applyRuntimeSnapshot: (runtimeSnapshot: SidebarForegroundRuntimeSnapshot) => boolean;
  refreshMessageById: (conversationId: string, messageId: string) => Promise<boolean>;
  finalizeMessage: (messageId: string) => void;
};

export type SidebarForegroundRecoveryOutcome = "handled" | "check_freshness" | "reload_conversation";

function normalized(value: unknown): string {
  return String(value || "").trim();
}

function runtimeIsStreaming(snapshot: SidebarForegroundRuntimeSnapshot): boolean {
  return normalized(snapshot.runtimeState) === "assistant_streaming";
}

function targetMessageId(snapshot: SidebarForegroundRuntimeSnapshot, fallbackMessageId?: string): string {
  return normalized(snapshot.streamCache?.persistedAssistantMessageId || fallbackMessageId);
}

function decide(
  input: SidebarForegroundRecoveryInput,
  snapshot: SidebarForegroundRuntimeSnapshot,
  probeState: "unknown" | "healthy" | "unhealthy",
) {
  return decideForegroundRecovery({
    backendStreaming: runtimeIsStreaming(snapshot),
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

export async function recoverSidebarForegroundStreaming(
  input: SidebarForegroundRecoveryInput,
  dependencies: SidebarForegroundRecoveryDependencies,
): Promise<SidebarForegroundRecoveryOutcome> {
  let action = decide(input, input.runtimeSnapshot, "unknown");
  if (action === "probe_stream") {
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    action = decide(input, input.runtimeSnapshot, probeHealthy ? "healthy" : "unhealthy");
  }

  if (action === "keep") {
    return input.frontendStreaming || runtimeIsStreaming(input.runtimeSnapshot)
      ? "handled"
      : "check_freshness";
  }

  if (action === "refresh_target_message") {
    const messageId = targetMessageId(input.runtimeSnapshot, input.frontendMessageId);
    if (!messageId || !await dependencies.refreshMessageById(input.conversationId, messageId)) {
      return "reload_conversation";
    }
    dependencies.finalizeMessage(messageId);
    return "handled";
  }

  if (action !== "resume_stream") return "reload_conversation";

  const resumedSnapshot = await dependencies.resumeSubscription(input.conversationId);
  const effectiveSnapshot = resumedSnapshot || input.runtimeSnapshot;
  const messageId = targetMessageId(effectiveSnapshot, input.frontendMessageId);
  if (runtimeIsStreaming(effectiveSnapshot)) {
    if (!messageId) return "reload_conversation";
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    if (!probeHealthy) return "reload_conversation";
    return dependencies.applyRuntimeSnapshot(effectiveSnapshot)
      ? "handled"
      : "reload_conversation";
  }

  if (!messageId || !await dependencies.refreshMessageById(input.conversationId, messageId)) {
    return "reload_conversation";
  }
  dependencies.finalizeMessage(messageId);
  return "handled";
}
