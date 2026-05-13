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
      <div class="max-h-[min(52vh,28rem)] overflow-y-auto px-4 py-3">
        <div class="whitespace-pre-wrap break-words text-sm leading-6 text-base-content/85">
          {{ text }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{
  visible: boolean;
  text: string;
}>();

defineEmits<{
  (e: "close"): void;
}>();

const { t } = useI18n();
</script>