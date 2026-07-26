import { messageHasVisibleContent } from "./use-chat-flow-utils";
import { preserveStableRenderId } from "../utils/stable-render-id";

const AUTHORITATIVE_PROVIDER_META_KEYS = [
  "contextUsagePercent",
  "contextUsageRatio",
  "effectivePromptTokens",
  "providerPromptTokens",
  "contextWindowTokens",
  "planCard",
];

function messageIsStreaming(message: any): boolean {
  return !!((message?.providerMeta || {}) as Record<string, unknown>)._streaming;
}

function mergeAuthoritativeProviderMeta(message: any, incomingMessage: any): any {
  const incomingProviderMeta = incomingMessage?.providerMeta;
  if (!incomingProviderMeta || typeof incomingProviderMeta !== "object") return message;
  const providerMeta = { ...(message?.providerMeta || {}) } as Record<string, unknown>;
  let changed = false;
  for (const key of AUTHORITATIVE_PROVIDER_META_KEYS) {
    if (!Object.prototype.hasOwnProperty.call(incomingProviderMeta, key)) continue;
    const nextValue = (incomingProviderMeta as Record<string, unknown>)[key];
    if (Object.is(providerMeta[key], nextValue)) continue;
    providerMeta[key] = nextValue;
    changed = true;
  }
  return changed ? { ...message, providerMeta } : message;
}

export function reconcileAuthoritativeConversationMessage(
  existingMessage: any,
  incomingMessage: any,
  options?: { forceReplace?: boolean },
): any {
  if (!existingMessage) return incomingMessage;
  if (options?.forceReplace || messageIsStreaming(existingMessage) || !messageHasVisibleContent(existingMessage)) {
    return preserveStableRenderId(incomingMessage, existingMessage);
  }
  return mergeAuthoritativeProviderMeta(existingMessage, incomingMessage);
}
