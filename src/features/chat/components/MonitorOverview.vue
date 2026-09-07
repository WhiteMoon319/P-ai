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
            <a :title="taskTitle(task)" @click="emit('switchPanelTab', 'tasks')">{{ taskTitle(task) }}</a>
          </li>
        </ul>
      </li>
      <li v-if="currentBatch">
        <a>{{ t("chat.toolReview.overviewLatestChanges") }} {{ currentBatch.itemCount }}</a>
        <ul>
          <li>
            <a :title="currentBatch.userMessageText" @click="emit('switchPanelTab', 'tools')">{{ currentBatch.userMessageText }}</a>
          </li>
        </ul>
      </li>
      <li v-if="runningBackgroundShells.length > 0">
        <a>{{ t("chat.toolReview.overviewBackgroundShells") }} {{ runningBackgroundShells.length }}</a>
        <ul>
          <li v-for="task in runningBackgroundShells" :key="task.id">
            <a class="flex items-center justify-between gap-2" :title="task.description">
              <span class="min-w-0 truncate">{{ task.description }}</span>
              <span class="shrink-0 text-xs tabular-nums text-base-content/55">{{ shellElapsedText(task.startedAt) }}</span>
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

// 后台终端快照不随事件逐秒更新，用组件内时钟驱动用时显示；无运行中 shell 时不空转
const shellClockNowMs = ref(Date.now());
let shellClockTimer: ReturnType<typeof window.setInterval> | null = null;

watch(
  () => runningBackgroundShells.value.length > 0,
  (shouldTick) => {
    if (shouldTick && shellClockTimer == null) {
      shellClockTimer = window.setInterval(() => {
        shellClockNowMs.value = Date.now();
      }, 1000);
    } else if (!shouldTick && shellClockTimer != null) {
      window.clearInterval(shellClockTimer);
      shellClockTimer = null;
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (shellClockTimer != null) {
    window.clearInterval(shellClockTimer);
    shellClockTimer = null;
  }
});

function shellElapsedText(startedAt: string) {
  const startedMs = Date.parse(String(startedAt || ""));
  if (!Number.isFinite(startedMs) || startedMs <= 0) return "";
  return formatDurationMs(shellClockNowMs.value - startedMs);
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
