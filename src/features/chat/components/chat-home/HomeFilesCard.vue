<template>
  <CardShell
    variant="wide"
    tone="info"
    :icon="Files"
    :label="t('chat.homePanel.openedFiles')"
    interactive
    @select="emit('openPanel')"
  >
    <ul v-if="files.length" class="menu menu-xs w-full gap-0.5 p-0">
      <li v-for="file in visibleFiles" :key="file.path">
        <button
          type="button"
          class="min-w-0 gap-2 font-normal"
          :title="file.path"
          @click.stop="emit('openFile', file.path)"
        >
          <span class="ecall-home-dot" :class="file.path === activePath ? 'bg-info' : 'bg-base-content/25'"></span>
          <span class="min-w-0 flex-1 truncate" :class="file.path === activePath ? 'text-base-content/90' : 'text-base-content/70'">
            {{ file.label }}
          </span>
        </button>
      </li>
    </ul>
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
</script>

<style scoped>
.ecall-home-dot {
  width: 0.25rem;
  height: 0.25rem;
  flex-shrink: 0;
  border-radius: 9999px;
}
</style>
