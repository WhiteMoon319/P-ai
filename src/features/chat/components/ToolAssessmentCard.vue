<template>
  <section class="last:mb-0 mx-1">
    <button
      type="button"
      class="group/card flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left transition-colors hover:bg-base-100/70"
      :disabled="loading"
      @click="openChanges"
    >
      <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-base-100 text-xs font-semibold text-base-content/65">
        #{{ item.orderIndex }}
      </div>
      <div class="min-w-0 flex-1 overflow-hidden">
        <div class="flex min-w-0 items-start justify-between gap-2">
          <span class="block min-w-0 flex-1 truncate whitespace-nowrap text-xs font-normal text-base-content">{{ title }}</span>
          <div v-if="timeDateLabel" class="shrink-0 text-right text-[11px] leading-4 text-base-content/55">{{ timeDateLabel }}</div>
        </div>
        <div class="mt-1 flex min-w-0 items-start justify-between gap-2 text-xs text-base-content/65">
          <div class="min-w-0 flex-1 truncate">{{ reviewOpinionText }}</div>
          <div v-if="timeMinuteLabel" class="shrink-0 text-right leading-4">
            {{ timeMinuteLabel }}
          </div>
        </div>
      </div>
      <span v-if="loading" class="loading loading-spinner loading-xs shrink-0 text-base-content/55"></span>
    </button>
  </section>

  <ToolReviewChangesDialog
    ref="changesDialogRef"
    :title="title"
    :subtitle="`#${item.orderIndex}`"
    :show-preview="!!detail"
    :preview-mode="detail?.previewKind === 'patch' ? 'patch' : 'plain'"
    :preview-text="detail ? detail.previewText || detail.resultText : ''"
    :review-opinion="dialogReviewOpinion"
    :review-allow="detail?.review?.allow"
    :review-model-name="detail?.review?.modelName || ''"
    :is-dark="isDark"
  />
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ToolReviewItemDetail, ToolReviewItemSummary } from "../composables/use-chat-tool-review";
import ToolReviewChangesDialog from "./ToolReviewChangesDialog.vue";
import { formatConversationListTimeWithMinuteDetails } from "../utils/conversation-time";

const props = withDefaults(defineProps<{
  item: ToolReviewItemSummary;
  detail?: ToolReviewItemDetail;
  loading: boolean;
  isDark?: boolean;
}>(), {
  detail: undefined,
  isDark: false,
});

const emit = defineEmits<{
  (e: "loadDetail", callId: string): void;
}>();

const { t, locale } = useI18n();
const changesDialogRef = ref<{ openChangesDialog: () => void; closeChangesDialog: () => void } | null>(null);
const pendingOpen = ref(false);

function isTerminalTool(toolName: string) {
  const normalized = String(toolName || "").trim();
  return normalized === "shell_exec" || normalized === "exec";
}

function isFileChangeTool(toolName: string) {
  const normalized = String(toolName || "").trim();
  return normalized === "apply_patch"
    || normalized === "write"
    || normalized === "delete"
    || normalized === "update"
    || normalized === "move";
}

const title = computed(() => {
  if (isTerminalTool(props.item.toolName)) {
    return String(props.item.command || "").trim() || t("chat.toolReview.terminalCommand");
  }
  if (isFileChangeTool(props.item.toolName)) {
    const fileName = patchFileName();
    const operation = patchOperationLabel(props.item.patchOperation);
    return fileName ? `${operation} ${fileName}` : operation;
  }
  return props.item.toolName;
});

const reviewOpinionText = computed(() => {
  const direct = props.detail?.review?.reviewOpinion;
  if (direct && direct.trim()) return direct;
  const summaryOpinion = String(props.item.reviewOpinion || "").trim();
  if (summaryOpinion) return summaryOpinion;
  if (props.item.hasReview && !props.detail) return t("chat.toolReview.evaluated");
  return props.item.hasReview ? t("chat.toolReview.reviewUnavailable") : t("chat.toolReview.unevaluated");
});

const dialogReviewOpinion = computed(() => String(props.detail?.review?.reviewOpinion || props.item.reviewOpinion || "").trim());
const timeParts = computed(() => {
  const raw = String(props.item.finishedAt || "").trim();
  return raw ? formatConversationListTimeWithMinuteDetails(raw, locale.value) : null;
});
const timeDateLabel = computed(() => timeParts.value?.dateLabel || "");
const timeMinuteLabel = computed(() => timeParts.value?.timeLabel || "");

watch(() => props.detail, (detail) => {
  if (!pendingOpen.value || !detail) return;
  pendingOpen.value = false;
  changesDialogRef.value?.openChangesDialog();
});

function openChanges() {
  if (props.detail) {
    changesDialogRef.value?.openChangesDialog();
    return;
  }
  pendingOpen.value = true;
  emit("loadDetail", props.item.callId);
}

function patchFileName() {
  const paths = Array.isArray(props.item.affectedPaths) ? props.item.affectedPaths : [];
  if (paths.length !== 1) return "";
  const normalized = String(paths[0] || "").replace(/\\/g, "/").trim();
  return normalized.split("/").filter(Boolean).pop() || "";
}

function patchOperationLabel(operation?: string) {
  if (operation === "add") return t("chat.toolReview.patchOperationAdd");
  if (operation === "delete") return t("chat.toolReview.patchOperationDelete");
  if (operation === "mixed") return t("chat.toolReview.patchOperationMixed");
  return t("chat.toolReview.patchOperationUpdate");
}
</script>
