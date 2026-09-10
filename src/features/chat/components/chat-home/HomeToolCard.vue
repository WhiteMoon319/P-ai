<template>
  <CardShell
    variant="wide"
    tone="info"
    :icon="Wrench"
    :label="t('chat.homePanel.toolLabel')"
    interactive
    @select="emit('open')"
  >
    <template #trailing>
      <span class="ecall-home-num shrink-0 text-xs text-base-content/45">{{ t("chat.homePanel.toolChangedFiles", { n: totalFileCount }) }}</span>
    </template>
    <div v-if="visibleFiles.length" class="flex min-h-0 flex-col overflow-hidden">
      <div v-for="file in visibleFiles" :key="file.path" class="min-w-0 truncate text-xs leading-4 text-base-content/80" :title="file.path">
        {{ file.label }}
      </div>
      <div v-if="moreCount > 0" class="truncate text-xs leading-4 text-base-content/35">
        {{ t("chat.homePanel.toolMoreFiles", { n: moreCount }) }}
      </div>
    </div>
  </CardShell>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Wrench } from "@lucide/vue";
import CardShell from "./CardShell.vue";
import type { ToolReviewBatchSummary } from "../../composables/use-chat-tool-review";

const MAX_VISIBLE_FILES = 4;

const props = withDefaults(defineProps<{
  /** 只含出现过工具调用的批次，按会话推进顺序（末尾最新），由 ChatHomePanel 过滤 */
  batches?: ToolReviewBatchSummary[];
}>(), {
  batches: () => [],
});

const emit = defineEmits<{
  (e: "open"): void;
}>();

const { t } = useI18n();

/** 卡片只讲最近一轮工具调用 */
const latestBatch = computed(() => (props.batches.length ? props.batches[props.batches.length - 1] : null));

/** 与后端 changed_files 同口径：只统计成功项涉及的文件，跨条目去重 */
const changedFilePaths = computed(() => {
  const batch = latestBatch.value;
  if (!batch) return [];
  const seen = new Set<string>();
  const paths: string[] = [];
  for (const item of batch.items || []) {
    if (item.isSuccess === false) continue;
    for (const rawPath of item.affectedPaths || []) {
      const normalized = String(rawPath || "").replace(/\\/g, "/").trim();
      if (!normalized || seen.has(normalized)) continue;
      seen.add(normalized);
      paths.push(normalized);
    }
  }
  return paths;
});

const totalFileCount = computed(() => {
  const declared = Number(latestBatch.value?.changedFiles);
  return Number.isFinite(declared) && declared > 0 ? declared : changedFilePaths.value.length;
});

const visibleFiles = computed(() =>
  changedFilePaths.value.slice(0, MAX_VISIBLE_FILES).map((path) => {
    const parts = path.split("/").filter(Boolean);
    return { path, label: parts.pop() || path };
  }),
);

const moreCount = computed(() => Math.max(totalFileCount.value - visibleFiles.value.length, 0));
</script>

<style scoped>
.ecall-home-num {
  font-variant-numeric: tabular-nums;
}
</style>
