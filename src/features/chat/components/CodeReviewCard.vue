<template>
  <section class="last:mb-0">
    <div class="group/card flex w-full items-center gap-2 rounded-2xl px-2 py-1 transition-colors hover:bg-base-200">
      <img v-if="avatarUrl" :src="avatarUrl" class="h-10 w-10 shrink-0 rounded-full object-cover" />
      <div v-else class="h-10 w-10 shrink-0 rounded-full bg-base-100"></div>
      <div class="min-w-0 flex-1 overflow-hidden">
        <span class="block truncate text-xs font-normal text-base-content">{{ title }}</span>
        <DelegateProgressLine
          v-if="running && progress"
          :running="true"
          :elapsed-ms="progress.elapsedMs"
          :request-count="progress.requestCount"
          :token-count="progress.tokenCount"
          :last-tool-name="progress.lastToolName"
        />
        <DelegateProgressLine v-else :text="summary" />
      </div>
      <button
        type="button"
        class="btn btn-ghost shrink-0 font-normal"
        @click.stop="emit('openDetail')"
      >{{ t("chat.toolReview.view") }}</button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ToolReviewReportRecord } from "../composables/use-chat-tool-review";
import DelegateProgressLine from "./DelegateProgressLine.vue";

defineProps<{
  report: ToolReviewReportRecord;
  title: string;
  summary: string;
  avatarUrl?: string;
  running?: boolean;
  progress?: {
    elapsedMs: number;
    requestCount: number;
    tokenCount: number;
    lastToolName: string;
  } | null;
}>();

const emit = defineEmits<{
  openDetail: [];
}>();

const { t } = useI18n();
</script>
