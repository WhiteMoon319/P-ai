<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type {
  TrimCompactionPreviewResult,
  TrimPreviewResult,
} from "../../composables/use-conversation-maintenance-dialog";

defineProps<{
  open: boolean;
  loading: boolean;
  running: boolean;
  preview: TrimPreviewResult | null;
  compactionPreview: TrimCompactionPreviewResult | null;
}>();

const emit = defineEmits<{
  close: [];
  confirmCompaction: [];
  confirmArchive: [];
  confirmDelete: [];
}>();

const { t } = useI18n();
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-[80vw] max-w-[80vw]">
      <div class="flex items-center justify-between gap-4">
        <h3 class="font-semibold text-base">{{ t("dialogs.trim.title") }}</h3>
        <div class="flex shrink-0 items-center gap-3 text-xs opacity-60">
          <span>{{ t("dialogs.trim.messageCount", { count: preview?.messageCount ?? 0 }) }}</span>
          <span>{{ t("dialogs.trim.contextUsage", { percent: compactionPreview?.contextUsagePercent ?? 0 }) }}</span>
        </div>
      </div>
      <div v-if="loading" class="mt-3 text-sm opacity-70">{{ t("dialogs.trim.loading") }}</div>
      <template v-else>
        <div class="mt-3 rounded-box border border-base-300 bg-base-200/40 px-3 py-3 text-sm">
          <div class="font-medium">{{ t("dialogs.trim.compactTitle") }}</div>
          <div class="mt-1 opacity-80">{{ t("dialogs.trim.compactSummary") }}</div>
          <div class="mt-2 text-xs opacity-70">{{ t("dialogs.trim.compactHint") }}</div>
          <div
            v-if="compactionPreview?.compactionDisabledReason"
            class="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning"
          >
            {{ compactionPreview.compactionDisabledReason }}
          </div>
        </div>
        <div class="mt-3 rounded-box border border-base-300 bg-base-200/40 px-3 py-3 text-sm">
          <div class="font-medium">{{ t("dialogs.trim.archiveTitle") }}</div>
          <div class="mt-1 opacity-80">{{ t("dialogs.trim.archiveSummary") }}</div>
          <div class="mt-2 text-xs opacity-70">{{ t("dialogs.trim.archiveHint") }}</div>
          <div
            v-if="preview?.archiveDisabledReason"
            class="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning"
          >
            {{ preview.archiveDisabledReason }}
          </div>
        </div>
      </template>
      <div class="mt-4 flex items-center justify-between gap-4">
        <div class="flex items-center gap-2">
          <button
            class="btn btn-sm btn-error"
            :disabled="loading || !preview?.canDropConversation || running"
            @click="emit('confirmDelete')"
          >
            {{ t("common.delete") }}
          </button>
          <button
            class="btn btn-sm btn-secondary"
            :disabled="loading || !preview?.canArchive || running"
            @click="emit('confirmArchive')"
          >
            {{ t("dialogs.trim.archiveTitle") }}
          </button>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="btn btn-sm btn-primary"
            :disabled="loading || !compactionPreview?.canCompact || running"
            @click="emit('confirmCompaction')"
          >
            {{ t("dialogs.trim.compactTitle") }}
          </button>
          <button class="btn btn-sm" :disabled="loading || running" @click="emit('close')">
            {{ t("common.cancel") }}
          </button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>
