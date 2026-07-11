import { computed, type Ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";
import { type ChatRenderItem, isRightAlignedMessage } from "../utils/chat-render";

export function useChatBlockTracking(
  messageBlocks: Ref<ChatMessageBlock[]>,
  chatRenderItems: Ref<ChatRenderItem[]>,
) {
  function isOwnMessage(block: ChatMessageBlock): boolean {
    return isRightAlignedMessage(block);
  }

  function isOwnUserMessage(block: ChatMessageBlock): boolean {
    if (block.remoteImOrigin) return false;
    const speakerAgentId = String(block.speakerAgentId || "").trim();
    return block.role === "user" || speakerAgentId === "user-persona";
  }

  function blockBelongsToMessageId(block: ChatMessageBlock, messageId: string): boolean {
    const normalizedMessageId = String(messageId || "").trim();
    if (!normalizedMessageId) return false;
    const sourceMessageId = String(block.sourceMessageId || "").trim();
    const blockId = String(block.id || "").trim();
    return sourceMessageId === normalizedMessageId || blockId === normalizedMessageId;
  }

  function findLatestMessageIdByPredicate(predicate: (block: ChatMessageBlock) => boolean): string {
    for (let idx = messageBlocks.value.length - 1; idx >= 0; idx -= 1) {
      const block = messageBlocks.value[idx];
      if (block.isExtraTextBlock) continue;
      if (!predicate(block)) continue;
      const messageId = String(block.sourceMessageId || block.id || "").trim();
      if (messageId) return messageId;
    }
    return "";
  }

  const latestOwnMessageId = computed(() => {
    return findLatestMessageIdByPredicate((block) => isOwnUserMessage(block));
  });

  const latestOwnElasticItemId = computed(() => {
    // 1) 有用户气泡时，始终盯最新用户消息
    const ownMessageId = latestOwnMessageId.value;
    if (ownMessageId) {
      for (let idx = chatRenderItems.value.length - 1; idx >= 0; idx -= 1) {
        const item = chatRenderItems.value[idx];
        if (item.kind === "message" && blockBelongsToMessageId(item.block, ownMessageId)) {
          return item.id;
        }
      }
    }

    // 2) 没有用户气泡时，在所有分割线里取时间线上最新的一个
    //    当前分割线只有 compaction / plan_started
    for (let idx = chatRenderItems.value.length - 1; idx >= 0; idx -= 1) {
      const item = chatRenderItems.value[idx];
      if (item.kind === "compaction" || item.kind === "plan_started") {
        return item.id;
      }
    }
    return "";
  });

  return {
    isOwnMessage,
    latestOwnMessageId,
    latestOwnElasticItemId,
  };
}
