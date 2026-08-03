<template>
  <div ref="rootRef" class="relative min-w-0" v-bind="$attrs">
    <!-- 触发按钮 -->
    <button
      type="button"
      class="select select-bordered flex w-full items-center justify-between gap-2 pr-3 text-left"
      :disabled="disabled || (tree.length === 0 && extraOptions.length === 0)"
      @click="toggle"
    >
      <span class="min-w-0 flex-1 truncate" :class="selectedLeaf ? '' : 'text-base-content/50'">
        {{ selectedLabel || placeholder }}
      </span>
      <ChevronDown class="h-4 w-4 shrink-0 opacity-70 transition-transform" :class="open ? 'rotate-180' : ''" />
    </button>
  </div>

  <!-- 下拉面板：Teleport 到 body + popover 顶层显示，避免被卡片 overflow 裁剪 -->
  <Teleport to="body">
    <div
      v-if="open && !disabled"
      ref="panelRef"
      popover="manual"
      class="fixed z-50 m-0 p-0 overflow-y-auto overflow-x-hidden rounded-box border border-base-300 bg-base-100 shadow-xl"
      :style="panelStyle"
    >
      <ApiConfigSelectionMenu
        :tree="tree"
        :selected-id="modelValue"
        :extra-options="extraOptions"
        :placeholder="placeholder"
        @select="selectValue"
      />
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ChevronDown } from "@lucide/vue";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem } from "../../../types/app";
import { buildApiConfigSelectionTree } from "../utils/api-config-selection-tree";
import ApiConfigSelectionMenu from "./ApiConfigSelectionMenu.vue";

defineOptions({ inheritAttrs: false });

const props = withDefaults(defineProps<{
  modelValue?: string;
  apiConfigs: ApiConfigItem[];
  extraOptions?: Array<{ id: string; label: string }>;
  placeholder?: string;
  disabled?: boolean;
}>(), {
  modelValue: "",
  extraOptions: () => [],
  placeholder: "",
  disabled: false,
});

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();

const { t } = useI18n();
const rootRef = ref<HTMLElement | null>(null);
const panelRef = ref<HTMLElement | null>(null);
const open = ref(false);
const panelStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
  width: "20rem",
  maxHeight: "80vh",
});
const tree = computed(() => buildApiConfigSelectionTree(props.apiConfigs, t));

const selectedLeaf = computed(() => tree.value
  .flatMap((provider) => provider.models)
  .flatMap((model) => model.leaves)
  .find((leaf) => leaf.id === props.modelValue) || null,
);

const selectedLabel = computed(() => {
  const extraOption = props.extraOptions.find((option) => option.id === props.modelValue);
  if (extraOption) return extraOption.label;
  const leaf = selectedLeaf.value;
  if (!leaf) return "";
  const provider = tree.value.find((candidate) => candidate.models.some((model) => model.leaves.some((item) => item.id === leaf.id)));
  const model = provider?.models.find((candidate) => candidate.leaves.some((item) => item.id === leaf.id));
  return [provider?.name, model?.name, leaf.label].filter(Boolean).join(" / ");
});

function toggle() {
  open.value = !open.value;
}

function selectValue(value: string) {
  emit("update:modelValue", value);
  open.value = false;
}

async function refreshPanelPosition() {
  if (!open.value) return;
  const trigger = rootRef.value;
  if (!trigger) return;
  await nextTick();
  const margin = 8;
  const gap = 8;
  const triggerRect = trigger.getBoundingClientRect();
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const preferredWidth = Math.max(Math.round(triggerRect.width), 320);
  const maxAllowedWidth = Math.max(220, viewportWidth - margin * 2);
  const width = Math.min(preferredWidth, maxAllowedWidth);
  const spaceAbove = Math.max(0, triggerRect.top - margin - gap);
  const spaceBelow = Math.max(0, viewportHeight - triggerRect.bottom - margin - gap);
  // 优先下方；下方更挤时才向上开
  const openUpward = spaceAbove > spaceBelow;
  const availableHeight = openUpward ? spaceAbove : spaceBelow;
  const maxHeight = Math.max(
    0,
    Math.min(Math.floor(viewportHeight * 0.8), Math.floor(availableHeight)),
  );
  const left = Math.min(
    Math.max(margin, triggerRect.left),
    Math.max(margin, viewportWidth - width - margin),
  );
  const maxHeightPx = `${Math.round(maxHeight)}px`;

  if (openUpward) {
    // 用 bottom 锚定触发器上方，避免 top 计算误差把面板顶出屏幕
    const bottom = Math.max(margin, viewportHeight - triggerRect.top + gap);
    panelStyle.value = {
      left: `${Math.round(left)}px`,
      right: "auto",
      top: "auto",
      bottom: `${Math.round(bottom)}px`,
      width: `${Math.round(width)}px`,
      maxWidth: `calc(100vw - ${margin * 2}px)`,
      maxHeight: maxHeightPx,
    };
  } else {
    const top = triggerRect.bottom + gap;
    panelStyle.value = {
      left: `${Math.round(left)}px`,
      right: "auto",
      top: `${Math.round(top)}px`,
      bottom: "auto",
      width: `${Math.round(width)}px`,
      maxWidth: `calc(100vw - ${margin * 2}px)`,
      maxHeight: maxHeightPx,
    };
  }
}

watch(open, (value) => {
  if (value) {
    nextTick(() => {
      void panelRef.value?.showPopover();
      void refreshPanelPosition();
    });
  }
});

function closeOnOutsidePointer(event: PointerEvent) {
  if (!open.value) return;
  const target = event.target as Node | null;
  if (!target) return;
  if (rootRef.value?.contains(target)) return;
  if (panelRef.value?.contains(target)) return;
  open.value = false;
}

onMounted(() => {
  document.addEventListener("pointerdown", closeOnOutsidePointer);
  window.addEventListener("resize", refreshPanelPosition);
  window.addEventListener("scroll", refreshPanelPosition, true);
});
onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", closeOnOutsidePointer);
  window.removeEventListener("resize", refreshPanelPosition);
  window.removeEventListener("scroll", refreshPanelPosition, true);
});
</script>
