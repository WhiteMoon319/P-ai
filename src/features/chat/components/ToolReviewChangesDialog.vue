<template>
  <dialog ref="dialogRef" class="modal">
    <div class="modal-box h-[90vh] w-[90vw] max-w-none p-0">
      <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
        <div class="min-w-0">
          <div class="truncate text-sm">{{ title }}</div>
          <div class="text-xs text-base-content/60">{{ subtitle }}</div>
        </div>
        <button
          type="button"
          class="btn btn-sm btn-ghost"
          @click="closeChangesDialog"
        >
          {{ t("chat.toolReview.closeChanges") }}
        </button>
      </div>
      <div class="flex h-[calc(90vh-61px)] min-h-0 flex-col overflow-hidden">
        <ToolReviewCodePreview
          v-if="showPreview"
          :mode="previewMode"
          :title="previewMode === 'patch' ? '' : t('chat.toolReview.commandPreview')"
          :code="previewText"
          :is-dark="isDark"
        />
        <ToolReviewCodePreview
          v-if="rawReview"
          mode="plain"
          :title="t('chat.toolReview.rawReview')"
          :code="rawReview"
          :is-dark="isDark"
        />
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button>{{ t("chat.toolReview.closeChanges") }}</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolReviewCodePreview from "./ToolReviewCodePreview.vue";

withDefaults(defineProps<{
  title: string;
  subtitle: string;
  showPreview: boolean;
  previewMode: "plain" | "patch";
  previewText: string;
  rawReview?: string;
  isDark?: boolean;
}>(), {
  rawReview: "",
  isDark: false,
});

const { t } = useI18n();
const dialogRef = ref<HTMLDialogElement | null>(null);

function openChangesDialog() {
  dialogRef.value?.showModal();
}

function closeChangesDialog() {
  dialogRef.value?.close();
}

defineExpose({
  openChangesDialog,
  closeChangesDialog,
});
</script>