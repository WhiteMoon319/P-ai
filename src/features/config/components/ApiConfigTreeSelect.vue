<template>
  <div ref="rootRef" class="relative min-w-0">
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

    <!-- 下拉面板：单层容器，避免嵌套空壳 -->
    <div
      v-if="open && !disabled"
      class="absolute z-50 mt-2 max-h-[80vh] w-full overflow-y-auto overflow-x-hidden rounded-box border border-base-300 bg-base-100 shadow-xl"
    >
      <ApiConfigSelectionMenu
        :tree="tree"
        :selected-id="modelValue"
        :extra-options="extraOptions"
        :placeholder="placeholder"
        @select="selectValue"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ChevronDown } from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem } from "../../../types/app";
import { buildApiConfigSelectionTree } from "../utils/api-config-selection-tree";
import ApiConfigSelectionMenu from "./ApiConfigSelectionMenu.vue";

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
const open = ref(false);
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

function closeOnOutsidePointer(event: PointerEvent) {
  if (!open.value) return;
  if (rootRef.value?.contains(event.target as Node)) return;
  open.value = false;
}

onMounted(() => document.addEventListener("pointerdown", closeOnOutsidePointer));
onBeforeUnmount(() => document.removeEventListener("pointerdown", closeOnOutsidePointer));
</script>
