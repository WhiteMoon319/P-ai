<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Copy, X } from "@lucide/vue";
import ToolReviewCodePreview from "../ToolReviewCodePreview.vue";

const props = defineProps<{
  open: boolean;
  lang?: string;
  code: string;
  isDark?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const { t } = useI18n();
const copied = ref(false);
let copiedTimer = 0;

const languageLabel = computed(() => String(props.lang || "").trim() || "text");

async function copyCode() {
  try {
    await navigator.clipboard.writeText(String(props.code || ""));
    copied.value = true;
    if (copiedTimer) window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      copied.value = false;
      copiedTimer = 0;
    }, 1200);
  } catch {
    copied.value = false;
  }
}

function requestClose() {
  emit("close");
}

onBeforeUnmount(() => {
  if (copiedTimer) window.clearTimeout(copiedTimer);
});
</script>

<template>
  <Teleport to="body">
    <dialog v-if="open" class="modal modal-open">
      <div class="modal-box flex h-[95vh] max-h-[95vh] w-[95vw] max-w-none flex-col overflow-hidden p-0">
        <div class="flex items-center gap-2 border-b border-base-300 px-4 py-2">
          <div class="min-w-0 flex-1 truncate text-xs font-semibold">{{ languageLabel }}</div>
          <button
            type="button"
            class="btn btn-ghost btn-xs gap-1"
            :title="copied ? '已复制' : t('common.copy')"
            @click="copyCode"
          >
            <Copy class="h-3.5 w-3.5" />
            <span>{{ copied ? "已复制" : t("common.copy") }}</span>
          </button>
          <button
            type="button"
            class="btn btn-ghost btn-square btn-xs"
            :title="t('common.close')"
            @click="requestClose"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
        <ToolReviewCodePreview
          class="min-h-0 flex-1"
          :code="code"
          :lang="languageLabel"
          :is-dark="isDark"
          show-line-numbers
        />
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click.prevent="requestClose">close</button>
      </form>
    </dialog>
  </Teleport>
</template>
