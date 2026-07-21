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
      <ul class="menu menu-sm w-full p-1">
        <li v-if="placeholder">
          <button
            type="button"
            class="text-sm"
            :class="!modelValue ? 'active' : ''"
            @click="selectValue('')"
          >
            <span class="min-w-0 flex-1 truncate">{{ placeholder }}</span>
          </button>
        </li>
        <li v-if="extraOptions.length > 0" class="menu-title px-2 pt-2">
          <div class="flex flex-wrap gap-1">
            <button
              v-for="option in extraOptions"
              :key="option.id"
              type="button"
              class="badge badge-outline rounded-full px-3 py-1.5 text-sm transition-colors hover:bg-base-200"
              :class="option.id === modelValue ? 'badge-primary bg-primary/10' : ''"
              @click="selectValue(option.id)"
            >
              {{ option.label }}
            </button>
          </div>
        </li>
        <template v-for="provider in tree" :key="provider.id">
          <li>
            <details>
              <summary class="font-semibold text-sm">
                <span class="min-w-0 flex-1 truncate">{{ provider.name }}</span>
                <span class="badge badge-ghost badge-xs shrink-0">{{ modelCount(provider) }}</span>
              </summary>
              <ul>
                <template v-for="model in provider.models" :key="model.key">
                  <li>
                    <details>
                      <summary class="text-sm">
                        <span class="min-w-0 flex-1 truncate">{{ model.name }}</span>
                        <template v-if="model.summaryFields.length > 0">
                          <span
                            v-for="field in model.summaryFields"
                            :key="field"
                            class="badge badge-outline badge-xs font-mono shrink-0"
                          >
                            {{ summaryValue(model.representative, field) }}
                          </span>
                        </template>
                      </summary>
                      <ul>
                        <li v-for="leaf in model.leaves" :key="leaf.id">
                          <button
                            type="button"
                            class="text-sm"
                            :class="leaf.id === modelValue ? 'active' : ''"
                            @click.stop="selectValue(leaf.id)"
                          >
                            <span class="min-w-0 flex-1 truncate">{{ leaf.label }}</span>
                            <span v-if="leaf.id === modelValue" class="badge badge-xs badge-primary shrink-0">✓</span>
                          </button>
                        </li>
                      </ul>
                    </details>
                  </li>
                </template>
              </ul>
            </details>
          </li>
        </template>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ChevronDown } from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem } from "../../../types/app";
import {
  apiConfigSelectionSummary,
  buildApiConfigSelectionTree,
  type ApiConfigSelectionSummaryField,
} from "../utils/api-config-selection-tree";

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

const summaryFieldLabels = computed<Record<ApiConfigSelectionSummaryField, string>>(() => ({
  contextWindowTokens: t("config.api.contextWindow"),
  maxOutputTokens: t("config.api.maxOutputTokens"),
  temperature: t("config.api.temperature"),
  enableTools: t("config.api.capTools"),
  enableImage: t("config.api.capImage"),
  enableAudio: t("config.api.capAudio"),
  enableVideo: t("config.api.capVideo"),
}));

function summaryValue(item: ApiConfigItem, field: ApiConfigSelectionSummaryField): string {
  return apiConfigSelectionSummary(item, [field], summaryFieldLabels.value);
}

function modelCount(provider: { models: Array<{ leaves: Array<{ id: string }> }> }): number {
  return provider.models.reduce((sum, model) => sum + model.leaves.length, 0);
}

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
