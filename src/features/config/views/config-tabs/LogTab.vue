<template>
  <div class="grid gap-3">
    <div class="card bg-base-100 border border-base-300">
      <div class="card-body p-4">
        <div class="flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div class="text-sm font-medium">{{ t("config.logs.title") }}</div>
            <div class="join">
              <button class="btn btn-sm bg-base-200 join-item" @click="props.openRuntimeLogs">
                {{ t("config.logs.backendLogs") }}
              </button>
              <button class="btn btn-sm bg-base-200 join-item" :disabled="loading" @click="reload">
                {{ t("common.refresh") }}
              </button>
              <button
                class="btn btn-sm bg-base-200 join-item"
                :disabled="loading || logs.length === 0"
                @click="clearAll"
              >
                {{ t("common.clear") }}
              </button>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2 text-sm">
            <span class="opacity-70">{{ t("config.logs.cacheSize") }}</span>
            <div class="join">
              <button
                v-for="option in logCapacityOptions"
                :key="option"
                class="btn btn-xs join-item"
                :class="props.config.llmRoundLogCapacity === option ? 'btn-primary' : 'bg-base-200'"
                type="button"
                @click="setLogCapacity(option)"
              >
                {{ t("config.logs.times", { count: option }) }}
              </button>
            </div>
          </div>
          <div class="text-sm opacity-60">
            {{ t("config.logs.capacityHint", { count: props.config.llmRoundLogCapacity }) }}
          </div>
        </div>
      </div>
    </div>

    <div v-if="loading" class="text-sm opacity-70">{{ t("common.loading") }}</div>
    <div v-else-if="logs.length === 0" class="text-sm opacity-50">{{ t("config.logs.noLogs") }}</div>

    <div v-else class="space-y-4">
      <div v-if="pipelineLogs.length" class="space-y-3">
        <div class="text-sm font-medium opacity-80">{{ t("config.logs.pipelineLogs") }}</div>
        <div
          v-for="entry in pipelineLogs"
          :key="entry.id"
          class="card bg-base-100 border-2 border-primary/20 shadow-sm"
        >
          <div class="card-body p-4 space-y-4">
            <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
              <div class="space-y-2">
                <div class="flex items-center gap-2 flex-wrap">
                  <div class="badge badge-primary badge-outline">{{ t("config.logs.currentPipeline") }}</div>
                  <div class="text-sm font-medium break-all">
                    {{ entry.createdAt }} | {{ entry.provider }} | {{ entry.requestFormat }} | {{ entry.model }}
                  </div>
                </div>
                <div class="text-xs opacity-60 break-all">{{ entry.baseUrl || "-" }}</div>
                <div v-if="entry.traceId" class="text-xs opacity-60 break-all">
                  trace: {{ entry.traceId }}
                </div>
              </div>
              <div class="badge" :class="entry.success ? 'badge-success' : 'badge-error'">
                {{ entry.success ? t("common.success") : t("common.failed") }}
              </div>
            </div>

            <div class="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
              <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
                <div class="text-xs opacity-60">{{ t("config.logs.totalElapsed") }}</div>
                <div class="text-sm font-medium">{{ entry.elapsedMs }}ms</div>
              </div>
              <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
                <div class="text-xs opacity-60">{{ t("config.logs.modelRounds") }}</div>
                <div class="text-sm font-medium">{{ entry.roundCount ?? (entry.rounds?.length ?? 0) }}</div>
              </div>
              <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
                <div class="text-xs opacity-60">{{ t("config.logs.toolCalls") }}</div>
                <div class="text-sm font-medium">{{ entry.toolCallCount ?? totalToolCallsForRounds(entry.rounds) }}</div>
              </div>
              <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
                <div class="text-xs opacity-60">{{ t("config.logs.errorSummary") }}</div>
                <div class="text-sm font-medium break-all">{{ entry.error?.trim() || "-" }}</div>
              </div>
            </div>

            <details
              v-if="entry.timeline?.length"
              class="collapse collapse-arrow bg-base-200 border border-base-300"
            >
              <summary class="collapse-title text-sm py-2 min-h-0">
                {{ t("config.logs.pipelineTimeline", { count: entry.timeline.length }) }}
              </summary>
              <div class="collapse-content text-sm space-y-2">
                <div class="opacity-70 break-all">
                  {{ t("config.logs.slowStages") }}
                  {{ topSlowStages(entry).map((item) => `${item.stage} +${item.sincePrevMs}ms`).join(" | ") || "-" }}
                </div>
                <pre class="whitespace-pre-wrap break-all">{{ toPretty(entry.timeline) }}</pre>
              </div>
            </details>

            <details class="collapse collapse-arrow bg-base-200 border border-base-300">
              <summary class="collapse-title text-sm py-2 min-h-0">{{ t("config.logs.pipelineResponse") }}</summary>
              <div class="collapse-content text-sm">
                <pre class="whitespace-pre-wrap break-all">{{ toPretty(entry.response ?? null) }}</pre>
              </div>
            </details>

            <div class="rounded-box border border-base-300 bg-base-200/60 p-3 space-y-3">
              <div class="flex items-center justify-between gap-2">
                <div class="text-sm font-medium">{{ t("config.logs.roundsTitle", { count: entry.rounds?.length ?? 0 }) }}</div>
                <div class="text-xs opacity-60">
                  {{ t("config.logs.roundsHint") }}
                </div>
              </div>
              <div v-if="entry.rounds?.length" class="space-y-2">
                <button
                  v-for="(round, index) in entry.rounds"
                  :key="round.id"
                  class="w-full rounded-box border border-base-300 bg-base-100 px-3 py-2 text-left transition hover:border-primary/40"
                  @click="openRound(entry, round, index)"
                >
                  <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
                    <div class="space-y-1">
                      <div class="text-sm font-medium">
                        {{ t("config.logs.roundTitle", { index: index + 1 }) }} | {{ round.provider }} | {{ round.model }}
                      </div>
                      <div class="text-xs opacity-60 break-all">{{ round.baseUrl || "-" }}</div>
                    </div>
                    <div class="flex items-center gap-2 flex-wrap">
                      <div class="badge badge-sm badge-outline">{{ round.elapsedMs }}ms</div>
                      <div class="badge badge-sm badge-outline">
                        {{ t("config.logs.toolCount", { count: toolCallCountForEntry(round) }) }}
                      </div>
                      <div class="badge badge-sm" :class="round.success ? 'badge-success' : 'badge-error'">
                        {{ round.success ? t("common.success") : t("common.failed") }}
                      </div>
                    </div>
                  </div>
                </button>
              </div>
              <div v-else class="text-sm opacity-60">{{ t("config.logs.noRoundDetails") }}</div>
            </div>
          </div>
        </div>
      </div>

      <div v-if="otherLogs.length" class="space-y-2">
        <div class="text-sm font-medium opacity-80">{{ t("config.logs.otherRequests") }}</div>
        <div
          v-for="entry in otherLogs"
          :key="entry.id"
          class="card bg-base-100 border border-base-300"
        >
          <div class="card-body p-3 space-y-2">
            <div class="flex items-center justify-between gap-2">
              <div class="text-sm opacity-70 break-all">
                {{ entry.createdAt }} | {{ entry.scene }} | {{ entry.provider }} | {{ entry.requestFormat }} | {{ entry.model }}
              </div>
              <div class="badge badge-sm" :class="entry.success ? 'badge-success' : 'badge-error'">
                {{ entry.success ? t("common.success") : t("common.failed") }}
              </div>
            </div>

            <div class="text-sm opacity-70">{{ t("config.logs.elapsed", { ms: entry.elapsedMs }) }} | {{ entry.baseUrl || "-" }}</div>
            <div v-if="entry.traceId" class="text-xs opacity-60 break-all">trace: {{ entry.traceId }}</div>

            <details
              v-if="entry.timeline?.length"
              class="collapse collapse-arrow bg-base-200 border border-base-300"
            >
              <summary class="collapse-title text-sm py-2 min-h-0">
                {{ t("config.logs.timeline", { count: entry.timeline.length }) }}
              </summary>
              <div class="collapse-content text-sm space-y-2">
                <div class="opacity-70 break-all">
                  {{ t("config.logs.slowStages") }}
                  {{ topSlowStages(entry).map((item) => `${item.stage} +${item.sincePrevMs}ms`).join(" | ") || "-" }}
                </div>
                <pre class="whitespace-pre-wrap break-all">{{ toPretty(entry.timeline) }}</pre>
              </div>
            </details>

            <details class="collapse collapse-arrow bg-base-200 border border-base-300">
              <summary class="collapse-title text-sm py-2 min-h-0">Headers</summary>
              <div class="collapse-content text-sm">
                <pre class="whitespace-pre-wrap break-all">{{ toPretty(entry.headers) }}</pre>
              </div>
            </details>

            <details class="collapse collapse-arrow bg-base-200 border border-base-300">
              <summary class="collapse-title text-sm py-2 min-h-0">Tools</summary>
              <div class="collapse-content text-sm">
                <pre class="whitespace-pre-wrap break-all">{{ toPretty(entry.tools ?? null) }}</pre>
              </div>
            </details>

            <details class="collapse collapse-arrow bg-base-200 border border-base-300">
              <summary class="collapse-title text-sm py-2 min-h-0">Response</summary>
              <div class="collapse-content text-sm">
                <pre class="whitespace-pre-wrap break-all">{{ toPretty(entry.response ?? null) }}</pre>
              </div>
            </details>

            <div v-if="entry.error" class="text-sm text-error break-all">{{ entry.error }}</div>
          </div>
        </div>
      </div>
    </div>

    <dialog class="modal" :class="{ 'modal-open': !!selectedRound }">
      <div class="modal-box max-w-5xl space-y-4">
        <div class="flex items-start justify-between gap-3">
          <div class="space-y-1">
            <div class="text-lg font-semibold">
              {{ selectedRound ? t("config.logs.roundCallTitle", { index: selectedRound.index + 1 }) : t("config.logs.roundDetails") }}
            </div>
            <div v-if="selectedRound" class="text-sm opacity-70 break-all">
              {{ selectedRound.round.provider }} | {{ selectedRound.round.requestFormat }} | {{ selectedRound.round.model }}
            </div>
            <div v-if="selectedRound?.round.traceId" class="text-xs opacity-60 break-all">
              trace: {{ selectedRound.round.traceId }}
            </div>
          </div>
          <button class="btn btn-sm btn-ghost" @click="closeRound">{{ t("common.close") }}</button>
        </div>

        <div v-if="selectedRound" class="grid gap-2 sm:grid-cols-3">
          <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
            <div class="text-xs opacity-60">{{ t("config.logs.roundElapsed") }}</div>
            <div class="text-sm font-medium">{{ selectedRound.round.elapsedMs }}ms</div>
          </div>
          <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
            <div class="text-xs opacity-60">{{ t("config.logs.toolCalls") }}</div>
            <div class="text-sm font-medium">{{ toolCallCountForEntry(selectedRound.round) }}</div>
          </div>
          <div class="rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
            <div class="text-xs opacity-60">{{ t("config.logs.status") }}</div>
            <div class="text-sm font-medium">{{ selectedRound.round.success ? t("common.success") : t("common.failed") }}</div>
          </div>
        </div>

        <div class="tabs tabs-boxed bg-base-200 inline-flex">
          <button
            v-for="tab in roundDetailTabs"
            :key="tab.id"
            class="tab"
            :class="{ 'tab-active': activeRoundTab === tab.id }"
            @click="activeRoundTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>

        <div v-if="selectedRound" class="rounded-box border border-base-300 bg-base-200/60 p-3">
          <pre
            v-if="activeRoundTab !== 'error'"
            class="whitespace-pre-wrap break-all text-sm"
          >{{ roundTabContent(selectedRound.round, activeRoundTab) }}</pre>
          <div v-else class="text-sm break-all" :class="selectedRound.round.error ? 'text-error' : 'opacity-60'">
            {{ selectedRound.round.error?.trim() || t("config.logs.noError") }}
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop" @submit.prevent="closeRound">
        <button @click="closeRound">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import type { AppConfig, LlmRoundLogEntry } from "../../../../types/app";
import { toErrorMessage } from "../../../../utils/error";

const props = defineProps<{
  config: AppConfig;
  openRuntimeLogs: () => void;
}>();

const { t } = useI18n();
const loading = ref(false);
const logs = ref<LlmRoundLogEntry[]>([]);
const logCapacityOptions = [1, 3, 10] as const;
const selectedRound = ref<{
  pipeline: LlmRoundLogEntry;
  round: LlmRoundLogEntry;
  index: number;
} | null>(null);
const roundDetailTabs = [
  { id: "response", label: "Response" },
  { id: "tools", label: "Tools" },
  { id: "headers", label: "Headers" },
  { id: "error", label: "Error" },
] as const;
const activeRoundTab = ref<(typeof roundDetailTabs)[number]["id"]>("response");

const pipelineLogs = computed(() =>
  logs.value.filter((entry) => entry.scene === "chat_pipeline"),
);

const otherLogs = computed(() =>
  logs.value.filter((entry) => entry.scene !== "chat_pipeline"),
);

function setLogCapacity(value: 1 | 3 | 10) {
  props.config.llmRoundLogCapacity = value;
}

function toPretty(input: unknown): string {
  try {
    return JSON.stringify(input, null, 2);
  } catch {
    return String(input ?? "");
  }
}

function topSlowStages(entry: LlmRoundLogEntry) {
  return [...(entry.timeline ?? [])]
    .sort((a, b) => b.sincePrevMs - a.sincePrevMs)
    .slice(0, 3);
}

function toolCallCountForEntry(entry: LlmRoundLogEntry): number {
  const response = entry.response as {
    toolCalls?: unknown[];
    toolHistoryEvents?: Array<{ tool_calls?: unknown[] }>;
  } | null | undefined;
  if (Array.isArray(response?.toolCalls)) {
    return response.toolCalls.length;
  }
  return (response?.toolHistoryEvents ?? []).reduce((total, item) => {
    return total + (Array.isArray(item?.tool_calls) ? item.tool_calls.length : 0);
  }, 0);
}

function totalToolCallsForRounds(rounds?: LlmRoundLogEntry[]): number {
  return (rounds ?? []).reduce((total, round) => total + toolCallCountForEntry(round), 0);
}

function openRound(pipeline: LlmRoundLogEntry, round: LlmRoundLogEntry, index: number) {
  selectedRound.value = { pipeline, round, index };
  activeRoundTab.value = "response";
}

function closeRound() {
  selectedRound.value = null;
}

function roundTabContent(
  entry: LlmRoundLogEntry,
  tab: (typeof roundDetailTabs)[number]["id"],
): string {
  if (tab === "response") {
    return toPretty(entry.response ?? null);
  }
  if (tab === "tools") {
    return toPretty(entry.tools ?? null);
  }
  return toPretty(entry.headers);
}

async function reload() {
  loading.value = true;
  try {
    const list = await invokeTauri<LlmRoundLogEntry[]>("list_recent_llm_round_logs");
    logs.value = [...list].reverse();
  } catch (error) {
    logs.value = [
      {
        id: "error",
        createdAt: new Date().toISOString(),
        scene: "ui",
        requestFormat: "-",
        provider: "-",
        model: "-",
        baseUrl: "",
        headers: [],
        tools: null,
        response: null,
        error: toErrorMessage(error),
        elapsedMs: 0,
        success: false,
      },
    ];
  } finally {
    loading.value = false;
  }
}

async function clearAll() {
  loading.value = true;
  try {
    await invokeTauri<boolean>("clear_recent_llm_round_logs");
    logs.value = [];
    selectedRound.value = null;
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void reload();
});
</script>
