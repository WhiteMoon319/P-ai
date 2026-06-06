<template>
  <div class="flex h-full min-h-0 w-full flex-col gap-2">
    <div v-if="title">{{ title }}</div>
    <TerminalApprovalPatchSample
      v-if="mode === 'patch'"
      class="h-full min-h-0"
      :lines="normalizedLines"
      :diff-only="true"
      highlight-style="background"
      :show-prefixes="false"
      :class="isDark ? 'tool-review-code-preview-dark' : 'tool-review-code-preview-light'"
    />
    <pre v-else class="h-full min-h-0 overflow-auto rounded-box border border-base-300 bg-base-200/40 p-3 text-[12px] leading-6"><code>{{ code }}</code></pre>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import TerminalApprovalPatchSample from "../../shell/components/TerminalApprovalPatchSample.vue";

const props = defineProps<{
  title?: string;
  code: string;
  mode?: "plain" | "patch";
  isDark?: boolean;
}>();

const normalizedLines = computed(() =>
  String(props.code || "").replace(/\r/g, "").split("\n"),
);
</script>

<style scoped>
:deep(.mockup-code) {
  height: 100%;
  max-height: none;
  border-radius: 0;
  background-color: var(--color-base-100);
  color: var(--color-base-content);
  scrollbar-color: color-mix(in srgb, var(--color-base-content) 30%, transparent) transparent;
  scrollbar-width: thin;
}

:deep(.tool-review-code-preview-light code) {
  color: var(--color-base-content);
}

:deep(.tool-review-code-preview-dark code) {
  color: var(--color-base-content);
}

:deep(.mockup-code::-webkit-scrollbar) {
  width: 6px;
  height: 6px;
}

:deep(.mockup-code::-webkit-scrollbar-thumb) {
  background: color-mix(in srgb, var(--color-base-content) 30%, transparent);
  border-radius: 3px;
}

:deep(.mockup-code::-webkit-scrollbar-track) {
  background: transparent;
}
</style>
