<template>
  <div
    v-if="visible"
    class="pointer-events-none absolute inset-x-0 top-4 z-30 flex justify-center px-3"
  >
    <div class="pointer-events-auto w-full max-w-xl rounded-box border border-base-300 bg-base-100 shadow-xl">
      <div class="flex items-center justify-between gap-3 border-b border-base-300 px-4 py-3">
        <div class="text-sm font-semibold">{{ t("chat.viewSummary") }}</div>
        <button
          type="button"
          class="btn btn-ghost btn-xs"
          @click="$emit('close')"
        >
          {{ t("common.close") }}
        </button>
      </div>
      <div class="max-h-[min(60vh,34rem)] overflow-y-auto px-4 py-3">
        <AppMarkdownRenderer
          class="ecall-markdown-content max-w-none text-sm text-base-content/85"
          :text="text"
          :is-dark="isDark"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import AppMarkdownRenderer from "../markdown/AppMarkdownRenderer.vue";

defineProps<{
  visible: boolean;
  text: string;
  isDark: boolean;
}>();

defineEmits<{
  (e: "close"): void;
}>();

const { t } = useI18n();
</script>
