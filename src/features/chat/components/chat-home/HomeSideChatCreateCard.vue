<template>
  <CardShell
    layout="tile"
    tone="secondary"
    :icon="MessageSquarePlus"
    :label="t('chat.sideChat.create')"
    interactive
    @select="emit('create')"
  />
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { MessageSquarePlus } from "@lucide/vue";
import CardShell from "./CardShell.vue";

const emit = defineEmits<{
  (e: "create"): void;
}>();

const { t } = useI18n();
</script>
