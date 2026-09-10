<template>
  <CardShell
    tone="neutral"
    :icon="ListTodo"
    :label="t('chat.homePanel.taskLabel')"
    interactive
    @select="emit('open')"
  >
    <span class="line-clamp-2 text-sm leading-snug text-base-content/85">{{ heading }}</span>
    <span v-if="nextRunText" class="mt-auto truncate text-xs text-base-content/45">{{ nextRunText }}</span>
  </CardShell>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { ListTodo } from "@lucide/vue";
import CardShell from "./CardShell.vue";

const props = withDefaults(defineProps<{
  goal?: string;
  nextRunAt?: string;
  /** 由容器统一驱动的时钟，避免每张卡片各起一个定时器 */
  nowMs?: number;
}>(), {
  goal: "",
  nextRunAt: "",
  nowMs: 0,
});

const emit = defineEmits<{
  (e: "open"): void;
}>();

const { t } = useI18n();

const heading = computed(() => String(props.goal || "").trim() || t("config.task.noTodo"));

const nextRunText = computed(() => {
  const targetMs = Date.parse(String(props.nextRunAt || "").trim());
  const nowMs = Number(props.nowMs || 0);
  if (!Number.isFinite(targetMs) || targetMs <= 0 || !nowMs) return "";
  const minutes = Math.max(1, Math.floor((targetMs - nowMs) / 60000));
  if (minutes < 60) return t("chat.toolReview.overviewNextRunMinutes", { n: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("chat.toolReview.overviewNextRunHours", { n: hours });
  return t("chat.toolReview.overviewNextRunDays", { n: Math.floor(hours / 24) });
});
</script>
