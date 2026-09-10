<template>
  <CardShell
    variant="wide"
    tone="info"
    :icon="Files"
    :label="t('chat.homePanel.openedFiles')"
    interactive
    @select="emit('openPanel')"
  >
    <template #trailing>
      <span class="shrink-0 text-xs text-base-content/45">{{ summary }}</span>
    </template>
    <div v-if="files.length" class="flex min-h-0 flex-col gap-0.5 overflow-hidden">
      <button
        v-for="file in visibleFiles"
        :key="file.path"
        type="button"
        class="flex min-w-0 items-center gap-2 rounded-md px-1 py-0.5 text-left text-xs transition-colors hover:bg-base-content/5"
        :title="file.path"
        @click.stop="emit('openFile', file.path)"
      >
        <span class="ecall-home-dot" :class="file.path === activePath ? 'bg-info' : 'bg-base-content/25'"></span>
        <span class="min-w-0 flex-1 truncate" :class="file.path === activePath ? 'text-base-content/90' : 'text-base-content/70'">
          {{ file.label }}
        </span>
      </button>
    </div>
    <div v-else class="flex flex-1 items-center justify-center text-xs text-base-content/40">
      {{ t("chat.homePanel.noOpenFile") }}
    </div>
  </CardShell>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Files } from "@lucide/vue";
import CardShell from "./CardShell.vue";

const MAX_VISIBLE = 4;

const props = withDefaults(defineProps<{
  files?: Array<{ path: string; label: string }>;
  activePath?: string;
  itemCount?: number;
}>(), {
  files: () => [],
  activePath: "",
  itemCount: 0,
});

const emit = defineEmits<{
  (e: "openPanel"): void;
  (e: "openFile", path: string): void;
}>();

const { t } = useI18n();

const visibleFiles = computed(() => props.files.slice(0, MAX_VISIBLE));
const summary = computed(() => (props.itemCount ? t("chat.homePanel.openFileCount", { n: props.itemCount }) : ""));
</script>

<style scoped>
.ecall-home-dot {
  width: 0.25rem;
  height: 0.25rem;
  flex-shrink: 0;
  border-radius: 9999px;
}
</style>
