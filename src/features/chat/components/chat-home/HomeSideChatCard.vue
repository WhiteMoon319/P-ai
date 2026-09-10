<template>
  <CardShell
    tone="secondary"
    :icon="MessageSquareMore"
    :label="t('chat.sideChat.title')"
    interactive
    @select="emit('open')"
  >
    <span class="line-clamp-2 text-xs leading-snug text-base-content/85">{{ title }}</span>
    <ArrowUpRight class="mt-auto size-3.5 self-end text-base-content/25" aria-hidden="true" />
  </CardShell>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { ArrowUpRight, MessageSquareMore } from "@lucide/vue";
import CardShell from "./CardShell.vue";

defineProps<{
  title: string;
}>();

const emit = defineEmits<{
  (e: "open"): void;
}>();

const { t } = useI18n();
</script>
