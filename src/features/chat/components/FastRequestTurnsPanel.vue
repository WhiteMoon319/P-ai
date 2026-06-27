<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="flex items-center justify-between gap-2 px-3 py-2">
      <div class="min-w-0">
        <div class="text-sm font-semibold text-base-content">{{ t("chat.fastRequest.title") }}</div>
        <div class="text-xs text-base-content/55">{{ countLabel }}</div>
      </div>
      <button
        type="button"
        class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8"
        :disabled="loading || !normalizedConversationId"
        :title="t('chat.fastRequest.refresh')"
        @click="loadTurns"
      >
        <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
      </button>
    </div>

    <div v-if="errorText" class="mx-4 my-3 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
      {{ errorText }}
    </div>

    <div v-else-if="!normalizedConversationId" class="flex min-h-0 flex-1 items-center justify-center px-4 py-8 text-sm text-base-content/65">
      {{ t("chat.fastRequest.noConversation") }}
    </div>

    <div v-else-if="loading && sortedTurns.length === 0" class="flex min-h-0 flex-1 items-center justify-center px-4 py-8">
      <span class="loading loading-spinner loading-sm text-base-content/45"></span>
    </div>

    <div v-else-if="sortedTurns.length === 0" class="flex min-h-0 flex-1 items-center justify-center px-4 py-8 text-sm text-base-content/65">
      {{ t("chat.fastRequest.empty") }}
    </div>

    <div v-else class="space-y-2 px-2 pb-3">
      <section
        v-for="turn in sortedTurns"
        :key="turn.id || `${turn.createdAt}:${turn.kind}`"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex min-w-0 items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="flex min-w-0 items-center gap-2">
              <span
                class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full"
                :class="turn.success ? 'bg-success/12 text-success' : 'bg-error/12 text-error'"
              >
                <CheckCircle2 v-if="turn.success" class="h-3.5 w-3.5" />
                <XCircle v-else class="h-3.5 w-3.5" />
              </span>
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-base-content">{{ kindLabel(turn.kind) }}</div>
                <div class="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-base-content/55">
                  <span>{{ turn.success ? t("chat.fastRequest.success") : t("chat.fastRequest.failed") }}</span>
                  <span v-if="turn.modelName" class="truncate">{{ turn.modelName }}</span>
                  <span v-if="turn.durationMs !== null && turn.durationMs !== undefined" class="inline-flex items-center gap-1">
                    <Clock3 class="h-3 w-3" />
                    {{ durationLabel(turn.durationMs) }}
                  </span>
                </div>
              </div>
            </div>
          </div>
          <div class="shrink-0 text-right text-[11px] leading-4 text-base-content/55">
            {{ timeLabel(turn.createdAt) }}
          </div>
        </div>

        <details class="mt-3 rounded-lg bg-base-200/70">
          <summary class="cursor-pointer px-2 py-1.5 text-xs font-medium text-base-content/70">
            {{ t("chat.fastRequest.request") }}
          </summary>
          <pre class="max-h-48 overflow-auto whitespace-pre-wrap break-words px-2 pb-2 text-xs leading-5 text-base-content/75">{{ displayText(turn.requestText) }}</pre>
        </details>

        <details class="mt-2 rounded-lg bg-base-200/70" :open="!turn.success">
          <summary class="cursor-pointer px-2 py-1.5 text-xs font-medium text-base-content/70">
            {{ turn.success ? t("chat.fastRequest.response") : t("chat.fastRequest.error") }}
          </summary>
          <pre class="max-h-48 overflow-auto whitespace-pre-wrap break-words px-2 pb-2 text-xs leading-5 text-base-content/75">{{ displayText(turn.success ? turn.responseText : (turn.error || turn.responseText)) }}</pre>
        </details>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { CheckCircle2, Clock3, RefreshCw, XCircle } from "@lucide/vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { FastRequestTurn } from "../../../types/app";
import { toErrorMessage } from "../../../utils/error";

const props = defineProps<{
  conversationId: string;
  active?: boolean;
  bridgeRequest?: <T = unknown>(method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<T>;
}>();

const { t, locale } = useI18n();
const turns = ref<FastRequestTurn[]>([]);
const loading = ref(false);
const errorText = ref("");
let requestSeq = 0;

const normalizedConversationId = computed(() => String(props.conversationId || "").trim());

const sortedTurns = computed(() =>
  turns.value
    .slice()
    .sort((left, right) => timestamp(right.createdAt) - timestamp(left.createdAt)),
);

const countLabel = computed(() => {
  const count = sortedTurns.value.length;
  return t("chat.fastRequest.count", { count });
});

async function loadTurns() {
  const conversationId = normalizedConversationId.value;
  const seq = ++requestSeq;
  errorText.value = "";
  if (!conversationId) {
    turns.value = [];
    return;
  }
  loading.value = true;
  try {
    const result = props.bridgeRequest
      ? await props.bridgeRequest<FastRequestTurn[]>("conversation.fastRequestTurns", { conversationId }, 10000)
      : await invokeTauri<FastRequestTurn[]>("get_conversation_fast_request_turns", {
          input: { conversationId },
        });
    if (seq !== requestSeq) return;
    turns.value = Array.isArray(result) ? result.map(normalizeTurn) : [];
  } catch (error) {
    if (seq !== requestSeq) return;
    errorText.value = t("chat.fastRequest.loadFailed", { error: toErrorMessage(error) });
  } finally {
    if (seq === requestSeq) {
      loading.value = false;
    }
  }
}

function normalizeTurn(turn: FastRequestTurn): FastRequestTurn {
  return {
    id: String(turn?.id || ""),
    kind: String(turn?.kind || ""),
    requestText: String(turn?.requestText || ""),
    responseText: String(turn?.responseText || ""),
    success: !!turn?.success,
    error: turn?.error ? String(turn.error) : null,
    modelName: turn?.modelName ? String(turn.modelName) : null,
    durationMs: turn?.durationMs === null || turn?.durationMs === undefined
      ? null
      : (Number.isFinite(Number(turn.durationMs)) ? Number(turn.durationMs) : null),
    createdAt: String(turn?.createdAt || ""),
  };
}

function kindLabel(kind: string) {
  const normalized = String(kind || "").trim();
  if (normalized === "remote_im") return t("chat.fastRequest.kindRemoteIm");
  if (normalized === "title_generation") return t("chat.fastRequest.kindTitleGeneration");
  if (normalized === "task_optimization") return t("chat.fastRequest.kindTaskOptimization");
  if (normalized === "tool_review") return t("chat.fastRequest.kindToolReview");
  return normalized || t("chat.fastRequest.unknownKind");
}

function durationLabel(value: number | null | undefined) {
  const ms = Number(value);
  if (!Number.isFinite(ms) || ms < 0) return "";
  return t("chat.fastRequest.durationMs", { ms: Math.round(ms) });
}

function timeLabel(raw: string) {
  const value = String(raw || "").trim();
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function timestamp(raw: string) {
  const time = new Date(String(raw || "")).getTime();
  return Number.isFinite(time) ? time : 0;
}

function displayText(text: string | null | undefined) {
  const value = String(text || "").trim();
  return value || "-";
}

watch(
  () => [props.active !== false, normalizedConversationId.value] as const,
  ([active]) => {
    if (!active) return;
    void loadTurns();
  },
  { immediate: true },
);
</script>
