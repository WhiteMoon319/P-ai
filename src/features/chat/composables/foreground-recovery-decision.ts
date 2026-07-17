export type ForegroundRecoveryProbeState = "unknown" | "healthy" | "unhealthy";

export type ForegroundRecoveryInput = {
  backendStreaming: boolean;
  frontendStreaming: boolean;
  backendMessageId?: string;
  frontendMessageId?: string;
  backendActivationId?: string;
  frontendActivationId?: string;
  backendRequestId?: string;
  frontendRequestId?: string;
  backendRevision?: string;
  frontendRevision?: string;
  probeState?: ForegroundRecoveryProbeState;
};

export type ForegroundRecoveryAction =
  | "keep"
  | "probe_stream"
  | "resume_stream"
  | "refresh_target_message"
  | "reload_conversation";

function normalized(value: unknown): string {
  return String(value || "").trim();
}

function sameNonEmpty(left?: string, right?: string): boolean {
  const normalizedLeft = normalized(left);
  const normalizedRight = normalized(right);
  return !!normalizedLeft && !!normalizedRight && normalizedLeft === normalizedRight;
}

function compatibleIdentity(left?: string, right?: string): boolean {
  return !normalized(left) || !normalized(right) || sameNonEmpty(left, right);
}

export function decideForegroundRecovery(input: ForegroundRecoveryInput): ForegroundRecoveryAction {
  const backendMessageId = normalized(input.backendMessageId);
  const frontendMessageId = normalized(input.frontendMessageId);
  const probeState = input.probeState || "unknown";
  const messageIdentityMatches = compatibleIdentity(backendMessageId, frontendMessageId);
  const activationIdentityMatches = compatibleIdentity(input.backendActivationId, input.frontendActivationId);
  const requestIdentityMatches = compatibleIdentity(input.backendRequestId, input.frontendRequestId);
  const streamingTargetMissing = input.backendStreaming
    && input.frontendStreaming
    && (!!backendMessageId !== !!frontendMessageId);
  const revisionMatches = !normalized(input.backendRevision)
    || !normalized(input.frontendRevision)
    || sameNonEmpty(input.backendRevision, input.frontendRevision);

  if (!input.backendStreaming && !input.frontendStreaming) {
    return revisionMatches ? "keep" : backendMessageId ? "refresh_target_message" : "reload_conversation";
  }
  if (input.backendStreaming && !input.frontendStreaming) {
    return backendMessageId ? "resume_stream" : "reload_conversation";
  }
  if (!input.backendStreaming && input.frontendStreaming) {
    return frontendMessageId ? "refresh_target_message" : "reload_conversation";
  }
  if (streamingTargetMissing || !messageIdentityMatches || !activationIdentityMatches || !requestIdentityMatches) {
    return backendMessageId ? "resume_stream" : "reload_conversation";
  }
  if (probeState === "unknown") return "probe_stream";
  if (probeState === "healthy") return "keep";
  return backendMessageId ? "resume_stream" : "reload_conversation";
}
