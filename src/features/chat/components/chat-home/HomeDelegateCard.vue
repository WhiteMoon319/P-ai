<template>
  <CardShell
    tone="primary"
    :icon="Network"
    :label="t('chat.homePanel.delegateLabel')"
    interactive
    @select="emit('open')"
  >
    <span class="line-clamp-2 text-sm leading-snug text-base-content/85">{{ title }}</span>
    <DelegateProgressLine
      class="mt-auto min-w-0 truncate"
      :running="true"
      :elapsed-ms="elapsedMs"
      :request-count="requestCount"
      :token-count="tokenCount"
      :last-tool-name="lastToolName"
    />
  </CardShell>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { Network } from "@lucide/vue";
import CardShell from "./CardShell.vue";
import DelegateProgressLine from "../DelegateProgressLine.vue";

withDefaults(defineProps<{
  title: string;
  elapsedMs?: number;
  requestCount?: number;
  tokenCount?: number;
  lastToolName?: string;
}>(), {
  elapsedMs: 0,
  requestCount: 0,
  tokenCount: 0,
  lastToolName: "",
});

const emit = defineEmits<{
  (e: "open"): void;
}>();

const { t } = useI18n();
</script>
