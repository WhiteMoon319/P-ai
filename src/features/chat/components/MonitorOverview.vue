<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <ul v-if="hasAnyWork" class="menu bg-base-200 w-full">
      <li v-if="runningDelegates.length > 0">
        <a>{{ t("chat.toolReview.overviewDelegates") }} {{ runningDelegates.length }}</a>
        <ul>
          <li v-for="delegate in runningDelegates" :key="delegate.delegateId">
            <a class="flex flex-col items-start gap-0.5" :title="delegate.title || delegate.delegateId" @click="emit('switchPanelTab', 'delegate')">
              <span class="block min-w-0 truncate">{{ delegate.title || delegate.delegateId }}</span>
              <DelegateProgressLine
                class="min-w-0 truncate"
                :running="true"
                :elapsed-ms="delegate.elapsedMs"
                :request-count="delegate.requestCount"
                :token-count="delegate.tokenCount"
                :last-tool-name="delegate.lastToolName"
              />
            </a>
          </li>
        </ul>
      </li>
      <li v-if="runningTasks.length > 0">
        <a>{{ t("chat.toolReview.overviewTasks") }} {{ runningTasks.length }}</a>
        <ul>
          <li v-for="task in runningTasks" :key="task.taskId">
            <a class="flex flex-col items-start gap-0.5" :title="taskTitle(task)" @click="emit('switchPanelTab', 'tasks')">
              <span class="block min-w-0 truncate">{{ taskTitle(task) }}</span>
              <span v-if="taskNextRunText(task)" class="text-xs text-base-content/65">{{ taskNextRunText(task) }}</span>
            </a>
          </li>
        </ul>
      </li>
      <li v-if="currentBatch">
        <a>{{ t("chat.toolReview.overviewLatestChanges") }} {{ currentBatch.itemCount }}</a>
        <ul>
          <li>
            <a class="flex flex-col items-start gap-0.5" :title="currentBatch.userMessageText" @click="emit('switchPanelTab', 'tools')">
              <span class="block min-w-0 truncate">{{ currentBatch.userMessageText }}</span>
              <span class="text-xs text-base-content/65">{{ batchSummaryText }}</span>
            </a>
          </li>
        </ul>
      </li>
      <li v-if="runningBackgroundShells.length > 0">
        <a>{{ t("chat.toolReview.overviewBackgroundShells") }} {{ runningBackgroundShells.length }}</a>
        <ul>
          <li v-for="task in runningBackgroundShells" :key="task.id">
            <a class="flex flex-col items-start gap-0.5" :title="task.description">
              <span class="block min-w-0 truncate">{{ task.description }}</span>
              <span class="text-xs text-base-content/65">{{ shellElapsedText(task.startedAt) }}</span>
            </a>
          </li>
        </ul>
      </li>
    </ul>
    <div v-else class="flex flex-1 items-center justify-center p-8 text-center">{{ t("chat.toolReview.overviewEmptyAll") }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { BackgroundShellTaskSummary, ConversationDelegateStatusSummary } from "../../../types/app";
import type { ToolReviewBatchSummary } from "../composables/use-chat-tool-review";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import type { ChatMonitorPanelMode } from "../composables/chat-ui-layout-storage";
import DelegateProgressLine from "./DelegateProgressLine.vue";

const props = defineProps<{
  delegateStatuses: ConversationDelegateStatusSummary[];
  runningTasks: TaskEntry[];
  backgroundShells: BackgroundShellTaskSummary[];
  currentBatch: ToolReviewBatchSummary | null;
}>();

const emit = defineEmits<{
  (e: "switchPanelTab", tab: ChatMonitorPanelMode): void;
}>();

const { t } = useI18n();

const runningDelegates = computed(() =>
  props.delegateStatuses.filter((delegate) => {
    const status = String(delegate.status || "").trim();
    return delegate.active && (status === "running" || status === "delivered");
  }),
);

const runningBackgroundShells = computed(() =>
  props.backgroundShells.filter((task) => String(task.status || "").trim() === "running"),
);

const hasAnyWork = computed(
  () =>
    runningDelegates.value.length > 0 ||
    props.runningTasks.length > 0 ||
    props.currentBatch != null ||
    runningBackgroundShells.value.length > 0,
);

// 后台终端与任务倒计时不逐秒推送，用组件内时钟驱动显示；无倒计面条目时不空转
const countdownClockNowMs = ref(Date.now());
let countdownClockTimer: ReturnType<typeof window.setInterval> | null = null;

const batchFinishedAtMs = computed(() => {
  let latest = NaN;
  for (const item of props.currentBatch?.items || []) {
    const ms = Date.parse(String(item.finishedAt || ""));
    if (Number.isFinite(ms) && (!Number.isFinite(latest) || ms > latest)) latest = ms;
  }
  return latest;
});

const batchSummaryText = computed(() => {
  const batch = props.currentBatch;
  if (!batch) return "";
  const parts = [t("chat.toolReview.overviewBatchItems", { n: batch.itemCount })];
  const unreviewed = Number(batch.unreviewedCount) || 0;
  if (unreviewed > 0) parts.push(t("chat.toolReview.overviewBatchUnreviewed", { n: unreviewed }));
  const finishedMs = batchFinishedAtMs.value;
  if (Number.isFinite(finishedMs) && finishedMs > 0) {
    const minutes = Math.max(1, Math.floor((countdownClockNowMs.value - finishedMs) / 60000));
    if (minutes < 60) parts.push(t("chat.toolReview.overviewAgoMinutes", { n: minutes }));
    else if (minutes < 60 * 24) parts.push(t("chat.toolReview.overviewAgoHours", { n: Math.floor(minutes / 60) }));
    else parts.push(t("chat.toolReview.overviewAgoDays", { n: Math.floor(minutes / 1440) }));
  }
  return parts.join(" · ");
});

const hasCountdownWork = computed(
  () =>
    runningBackgroundShells.value.length > 0 ||
    props.runningTasks.some((task) => taskNextRunText(task) !== "") ||
    Number.isFinite(batchFinishedAtMs.value),
);

watch(
  hasCountdownWork,
  (shouldTick) => {
    if (shouldTick && countdownClockTimer == null) {
      countdownClockTimer = window.setInterval(() => {
        countdownClockNowMs.value = Date.now();
      }, 1000);
    } else if (!shouldTick && countdownClockTimer != null) {
      window.clearInterval(countdownClockTimer);
      countdownClockTimer = null;
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (countdownClockTimer != null) {
    window.clearInterval(countdownClockTimer);
    countdownClockTimer = null;
  }
});

function shellElapsedText(startedAt: string) {
  const startedMs = Date.parse(String(startedAt || ""));
  if (!Number.isFinite(startedMs) || startedMs <= 0) return "";
  return formatDurationMs(countdownClockNowMs.value - startedMs);
}

function taskNextRunText(task: TaskEntry) {
  const targetMs = Date.parse(String(task.trigger?.next_run_at || "").trim());
  if (!Number.isFinite(targetMs) || targetMs <= 0) return "";
  const minutes = Math.max(1, Math.floor((targetMs - countdownClockNowMs.value) / 60000));
  if (minutes < 60) return t("chat.toolReview.overviewNextRunMinutes", { n: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("chat.toolReview.overviewNextRunHours", { n: hours });
  return t("chat.toolReview.overviewNextRunDays", { n: Math.floor(hours / 24) });
}

function formatDurationMs(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0秒";
  const totalSeconds = Math.floor(value / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}时${minutes}分`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}

function taskTitle(task: TaskEntry) {
  return String(task.goal || "").trim() || t("config.task.noTodo");
}
</script>
