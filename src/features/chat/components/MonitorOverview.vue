<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="min-h-0 flex-1 overflow-y-auto px-2 py-1">
      <section class="py-1">
        <button
          type="button"
          class="flex w-full items-center gap-1.5 rounded px-1 py-0.5 text-left"
          @click="emit('switchPanelTab', 'delegate')"
        >
          <span class="text-xs font-semibold text-base-content">{{ t("chat.toolReview.overviewDelegates") }}</span>
          <span class="text-xs tabular-nums text-base-content/45">{{ runningDelegates.length }}</span>
        </button>
        <ul v-if="runningDelegates.length > 0" class="mt-0.5 list-disc pl-6">
          <li v-for="delegate in runningDelegates" :key="delegate.delegateId">
            <div class="flex w-full items-center gap-2">
              <button
                type="button"
                class="min-w-0 flex-1 truncate py-0.5 text-left text-xs text-base-content/80 transition-colors hover:text-base-content"
                :title="delegate.title || delegate.delegateId"
                @click="emit('switchPanelTab', 'delegate')"
              >
                {{ delegate.title || delegate.delegateId }}
              </button>
              <span class="shrink-0 py-0.5 text-xs tabular-nums text-base-content/55">{{ elapsedSince(delegate.startedAt) }}</span>
            </div>
          </li>
        </ul>
      </section>

      <section class="border-t border-base-300 py-2">
        <button
          type="button"
          class="flex w-full items-center gap-1.5 rounded px-1 py-0.5 text-left"
          @click="emit('switchPanelTab', 'tasks')"
        >
          <span class="text-xs font-semibold text-base-content">{{ t("chat.toolReview.overviewTasks") }}</span>
          <span class="text-xs tabular-nums text-base-content/45">{{ runningTasks.length }}</span>
        </button>
        <ul v-if="runningTasks.length > 0" class="mt-0.5 list-disc pl-6">
          <li v-for="task in runningTasks" :key="task.taskId">
            <button
              type="button"
              class="block w-full truncate py-0.5 text-left text-xs text-base-content/80 transition-colors hover:text-base-content"
              :title="taskTitle(task)"
              @click="emit('switchPanelTab', 'tasks')"
            >
              {{ taskTitle(task) }}
            </button>
          </li>
        </ul>
      </section>

      <section v-if="currentBatch" class="border-t border-base-300 py-2">
        <button
          type="button"
          class="flex w-full items-center gap-1.5 rounded px-1 py-0.5 text-left"
          @click="emit('switchPanelTab', 'tools')"
        >
          <span class="text-xs font-semibold text-base-content">{{ t("chat.toolReview.overviewLatestChanges") }}</span>
          <span class="text-xs tabular-nums text-base-content/45">{{ currentBatch.itemCount }}</span>
        </button>
        <ul class="mt-0.5 list-disc pl-6">
          <li>
            <button
              type="button"
              class="block w-full truncate py-0.5 text-left text-xs text-base-content/80 transition-colors hover:text-base-content"
              :title="currentBatch.userMessageText"
              @click="emit('switchPanelTab', 'tools')"
            >
              {{ currentBatch.userMessageText }}
            </button>
          </li>
          <li class="py-0.5 text-xs text-base-content/55">
            {{ t("chat.toolReview.overviewUnreviewed", { count: currentBatch.unreviewedCount }) }}
          </li>
        </ul>
      </section>

      <section class="border-t border-base-300 py-2">
        <div class="flex w-full items-center gap-1.5 px-1 py-0.5">
          <span class="text-xs font-semibold text-base-content">{{ t("chat.toolReview.overviewBackgroundShells") }}</span>
          <span class="text-xs tabular-nums text-base-content/45">{{ runningBackgroundShells.length }}</span>
        </div>
        <ul v-if="runningBackgroundShells.length > 0" class="mt-0.5 list-disc pl-6">
          <li v-for="task in runningBackgroundShells" :key="task.id" class="py-0.5">
            <div class="flex w-full items-center gap-2">
              <div class="min-w-0 flex-1 overflow-hidden">
                <div class="truncate text-xs text-base-content/80" :title="task.description">{{ task.description }}</div>
                <div v-if="(task.outputTail || '').trim()" class="truncate font-mono text-xs text-base-content/55" :title="task.outputTail">{{ task.outputTail }}</div>
              </div>
              <span class="shrink-0 text-xs tabular-nums text-base-content/55">{{ elapsedSince(task.startedAt) }}</span>
            </div>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
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

const now = ref(Date.now());
let elapsedTimer: number | undefined;

onMounted(() => {
  elapsedTimer = window.setInterval(() => {
    now.value = Date.now();
  }, 1000);
});

onBeforeUnmount(() => {
  if (elapsedTimer) window.clearInterval(elapsedTimer);
});

const runningDelegates = computed(() =>
  props.delegateStatuses.filter((delegate) => {
    const status = String(delegate.status || "").trim();
    return delegate.active && (status === "running" || status === "delivered");
  }),
);

const runningBackgroundShells = computed(() =>
  props.backgroundShells.filter((task) => String(task.status || "").trim() === "running"),
);

function elapsedSince(startedAt: string) {
  const raw = String(startedAt || "").trim();
  if (!raw) return "";
  const startedMs = Date.parse(raw);
  if (!Number.isFinite(startedMs)) return raw;
  return formatRecentRelativeTime(startedMs, now.value);
}

function formatRecentRelativeTime(startedMs: number, nowMs: number): string {
  const diffMs = Math.max(0, nowMs - startedMs);
  const seconds = Math.floor(diffMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (seconds < 60) return t("config.memory.justNow");
  if (minutes < 60) return t("config.memory.minutesAgo", { count: minutes });
  if (hours < 24) return t("config.memory.hoursAgo", { count: hours });
  if (days < 7) return t("config.memory.daysAgo", { count: days });

  const date = new Date(startedMs);
  const nowDate = new Date(nowMs);
  const monthDay = `${padTimePart(date.getMonth() + 1)}-${padTimePart(date.getDate())}`;
  const clock = `${padTimePart(date.getHours())}:${padTimePart(date.getMinutes())}`;
  if (date.getFullYear() === nowDate.getFullYear()) return `${monthDay} ${clock}`;
  return `${date.getFullYear()}-${monthDay}`;
}

function padTimePart(value: number): string {
  return String(value).padStart(2, "0");
}

function taskTitle(task: TaskEntry) {
  return String(task.goal || "").trim() || t("config.task.noTodo");
}
</script>
