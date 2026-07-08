<template>
  <button
    type="button"
    class="btn btn-ghost btn-sm shrink-0"
    :title="t('chat.rightPanelSwitcherTitle')"
    @click.stop="selectPanel(targetPanelMode)"
  >
    <ArrowLeftRight class="size-4 opacity-70" />
    <span class="truncate">{{ targetPanelLabel }}</span>
  </button>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ArrowLeftRight } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import type { ChatRightPanelMode } from "../composables/chat-ui-layout-storage";

type ChatMonitorPanelMode = Exclude<ChatRightPanelMode, "reader">;

const props = defineProps<{
  modelValue: ChatRightPanelMode;
  monitorValue: ChatMonitorPanelMode;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: ChatRightPanelMode): void;
}>();

const { t } = useI18n();

const targetPanelMode = computed<ChatRightPanelMode>(() =>
  props.modelValue === "reader" ? props.monitorValue : "reader",
);

const targetPanelLabel = computed(() =>
  targetPanelMode.value === "reader" ? t("chat.filePanelTab") : t("chat.monitorPanelTab"),
);

function selectPanel(value: ChatRightPanelMode) {
  emit("update:modelValue", value);
}
</script>
