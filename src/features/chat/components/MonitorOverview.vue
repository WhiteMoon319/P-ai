<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <ul v-if="hasAnyWork" class="menu bg-base-200 w-full">
        <li v-if="runningDelegates.length > 0">
          <a>{{ t("chat.toolReview.overviewDelegates") }} {{ runningDelegates.length }}</a>
          <ul>
            <li v-for="delegate in runningDelegates" :key="delegate.delegateId">
              <a :title="delegate.title || delegate.delegateId" @click="emit('switchPanelTab', 'delegate')">{{ delegate.title || delegate.delegateId }}</a>
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
              <a :title="task.description">{{ task.description }}</a>
            </li>
          </ul>
        </li>
      </ul>
    <div v-else class="flex flex-1 items-center justify-center p-8 text-center">{{ t("chat.toolReview.overviewEmptyAll") }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { BackgroundShellTaskSummary, ConversationDelegateStatusSummary } from "../../../types/app";
import type { ToolReviewBatchSummary } from "../composables/use-chat-tool-review";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import type { ChatMonitorPanelMode } from "../composables/chat-ui-layout-storage";

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

function taskTitle(task: TaskEntry) {
  return String(task.goal || "").trim() || t("config.task.noTodo");
}
</script>
