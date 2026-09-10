<template>
  <CardShell
    layout="tile"
    tone="warning"
    :icon="FolderTree"
    :label="t('chat.homePanel.workspace')"
    interactive
    @select="emit('open')"
  >
    <span class="w-full truncate text-xs text-base-content/45" :title="workspaceRootPath">{{ name }}</span>
  </CardShell>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { FolderTree } from "@lucide/vue";
import CardShell from "./CardShell.vue";

const props = withDefaults(defineProps<{
  workspaceRootPath?: string;
}>(), {
  workspaceRootPath: "",
});

const emit = defineEmits<{
  (e: "open"): void;
}>();

const { t } = useI18n();

const name = computed(() => {
  const normalized = String(props.workspaceRootPath || "").replace(/\\/g, "/").replace(/\/+$/, "");
  return normalized.split("/").filter(Boolean).pop() || normalized;
});
</script>
