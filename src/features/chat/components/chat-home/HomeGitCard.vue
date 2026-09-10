<template>
  <CardShell
    variant="wide"
    tone="warning"
    :icon="GitBranch"
    :label="branch || t('chat.homePanel.gitChanges')"
    interactive
    @select="emit('openChanges')"
  >
    <template #trailing>
      <span class="shrink-0 text-xs text-base-content/45">{{ summary }}</span>
    </template>
    <div v-if="changes.length" class="flex min-h-0 flex-col overflow-hidden">
      <div v-for="change in visibleChanges" :key="change.path" class="flex min-w-0 items-center gap-2 text-xs leading-4">
        <span class="ecall-home-status w-2.5" :class="statusClass(change.status)">{{ statusLabel(change.status) }}</span>
        <span class="min-w-0 flex-1 truncate text-base-content/80" :title="change.path">{{ baseName(change.path) }}</span>
      </div>
      <button
        v-if="hiddenChangeCount > 0"
        type="button"
        class="min-w-0 truncate text-left text-xs leading-4 text-base-content/45 transition-colors hover:text-base-content/80"
        @click.stop="emit('openChanges')"
      >
        {{ t("chat.homePanel.gitMoreFiles", { n: hiddenChangeCount }) }}
      </button>
    </div>
    <div v-else class="flex flex-1 items-center justify-center text-xs text-base-content/40">
      {{ workspaceRootPath ? t("chat.homePanel.noChanges") : t("chat.homePanel.noWorkspace") }}
    </div>
  </CardShell>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { GitBranch } from "@lucide/vue";
import CardShell from "./CardShell.vue";

const MAX_VISIBLE = 4;

const props = withDefaults(defineProps<{
  workspaceRootPath?: string;
  branch?: string;
  changes?: Array<{ path: string; status: string }>;
  changeCount?: number;
}>(), {
  workspaceRootPath: "",
  branch: "",
  changes: () => [],
  changeCount: 0,
});

const emit = defineEmits<{
  (e: "openChanges"): void;
}>();

const { t } = useI18n();

const visibleChanges = computed(() => props.changes.slice(0, MAX_VISIBLE));
/** 卡片只显示前 4 条，剩余条数按总数减已显示算，截断的大仓库也成立 */
const hiddenChangeCount = computed(() => {
  const total = Number(props.changeCount || 0) || props.changes.length;
  return Math.max(total - visibleChanges.value.length, 0);
});

const summary = computed(() => {
  if (!props.workspaceRootPath) return "";
  if (!Number(props.changeCount || 0)) return t("chat.homePanel.cleanWorktree");
  return t("chat.homePanel.changeCount", { n: Number(props.changeCount || 0) });
});

/** Git 状态码转单字母，与 Git 面板的语义保持一致。 */
function statusLabel(status: string): string {
  const code = String(status || "").trim().charAt(0).toUpperCase();
  if (code === "A" || code === "?") return "A";
  if (code === "D") return "D";
  if (code === "R") return "R";
  return "M";
}

function statusClass(status: string): string {
  const label = statusLabel(status);
  if (label === "A") return "text-success";
  if (label === "D") return "text-error";
  if (label === "R") return "text-info";
  return "text-warning";
}

function baseName(path: string): string {
  const normalized = String(path || "").replace(/\\/g, "/");
  return normalized.split("/").filter(Boolean).pop() || normalized;
}
</script>

<style scoped>
.ecall-home-status {
  flex-shrink: 0;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
</style>
