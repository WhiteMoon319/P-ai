import type { ChatMessage, ChatMessageBlock } from "../../../types/app";

export const STABLE_RENDER_ID_META_KEY = "_stableRenderId";

type MessageLike = {
  id?: string;
  sourceMessageId?: string;
  providerMeta?: ChatMessage["providerMeta"];
};

export function stableRenderIdFromProviderMeta(providerMeta?: ChatMessage["providerMeta"]): string {
  return String(((providerMeta || {}) as Record<string, unknown>)[STABLE_RENDER_ID_META_KEY] || "").trim();
}

export function stableRenderIdFromMessage(message?: MessageLike | null): string {
  if (!message) return "";
  return stableRenderIdFromProviderMeta(message.providerMeta) || String(message.id || message.sourceMessageId || "").trim();
}

export function stableRenderIdFromBlock(block?: ChatMessageBlock | null): string {
  if (!block) return "";
  return stableRenderIdFromProviderMeta(block.providerMeta);
}

export function providerMetaWithoutStableRenderId(providerMeta?: ChatMessage["providerMeta"]): Record<string, unknown> {
  const nextMeta = { ...((providerMeta || {}) as Record<string, unknown>) };
  delete nextMeta[STABLE_RENDER_ID_META_KEY];
  return nextMeta;
}

export function messageWithStableRenderId<T extends { providerMeta?: ChatMessage["providerMeta"] }>(
  message: T,
  stableRenderId: string,
): T {
  const normalizedStableRenderId = String(stableRenderId || "").trim();
  if (!normalizedStableRenderId) return message;
  const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
  if (String(providerMeta[STABLE_RENDER_ID_META_KEY] || "").trim() === normalizedStableRenderId) {
    return message;
  }
  return {
    ...message,
    providerMeta: {
      ...providerMeta,
      [STABLE_RENDER_ID_META_KEY]: normalizedStableRenderId,
    },
  };
}

export function messageWithoutStableRenderId<T extends { providerMeta?: ChatMessage["providerMeta"] }>(message: T): T {
  const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
  if (!(STABLE_RENDER_ID_META_KEY in providerMeta)) return message;
  return {
    ...message,
    providerMeta: providerMetaWithoutStableRenderId(message.providerMeta),
  };
}

export function preserveStableRenderId<T extends { providerMeta?: ChatMessage["providerMeta"] }>(
  nextMessage: T,
  previousMessage?: MessageLike | null,
): T {
  const stableRenderId = stableRenderIdFromProviderMeta(previousMessage?.providerMeta);
  return stableRenderId ? messageWithStableRenderId(nextMessage, stableRenderId) : nextMessage;
}
