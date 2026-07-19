<template>
  <div
    v-if="attachments.length > 0"
    :class="[
      'flex max-w-full flex-wrap gap-1.5 pt-1',
      align === 'end' ? 'justify-end' : 'justify-start',
    ]"
  >
    <ChatAttachmentItem
      v-for="(attachment, index) in attachments"
      :key="attachment.id || `${attachment.kind}-${index}-${attachment.label}`"
      :attachment="attachment"
      :interactive="interactiveKinds.includes(attachment.kind)"
      :playing="attachment.id === playingId"
      @activate="emit('activate', { attachment, index })"
    />
  </div>
</template>

<script setup lang="ts">
import { toRefs } from "vue";
import ChatAttachmentItem from "./ChatAttachmentItem.vue";
import type { ChatAttachmentKind, ChatAttachmentView } from "../utils/chat-attachment-display";

const props = withDefaults(defineProps<{
  attachments: ChatAttachmentView[];
  align?: "start" | "end";
  interactiveKinds?: ChatAttachmentKind[];
  playingId?: string;
}>(), {
  align: "start",
  interactiveKinds: () => [],
  playingId: "",
});

const emit = defineEmits<{
  (e: "activate", payload: { attachment: ChatAttachmentView; index: number }): void;
}>();

const { attachments, align, interactiveKinds, playingId } = toRefs(props);
</script>
