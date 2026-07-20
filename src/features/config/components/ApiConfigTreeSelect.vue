<template>
  <div ref="rootRef" class="relative min-w-0">
    <button
      type="button"
      class="select select-bordered flex w-full items-center justify-between gap-2 pr-3 text-left"
      :disabled="disabled || (tree.length === 0 && extraOptions.length === 0)"
      @click="open = !open"
    >
      <span class="min-w-0 flex-1 truncate" :class="selectedLeaf ? '' : 'text-base-content/50'">
        {{ selectedLabel || placeholder }}
      </span>
      <ChevronDown class="h-4 w-4 shrink-0 opacity-70 transition-transform" :class="open ? 'rotate-180' : ''" />
    </button>

    <div
      v-if="open && !disabled"
      class="absolute z-50 mt-2 max-h-96 w-full overflow-auto rounded-box border border-base-300 bg-base-100 p-2 shadow-xl"
    >
      <button
        v-if="placeholder"
        type="button"
        class="flex w-full rounded-lg px-3 py-2 text-left text-sm hover:bg-base-200"
        :class="!modelValue ? 'bg-primary/10' : ''"
        @click="selectValue('')"
      >
        {{ placeholder }}
      </button>
      <section v-if="extraOptions.length > 0" class="mb-2 flex flex-wrap gap-1">
        <button
          v-for="option in extraOptions"
          :key="option.id"
          type="button"
          class="rounded-md px-3 py-1.5 text-sm hover:bg-base-200"
          :class="option.id === modelValue ? 'bg-primary/10 font-medium text-primary' : 'bg-base-200/60'"
          @click="selectValue(option.id)"
        >
          {{ option.label }}
        </button>
      </section>
      <section v-for="provider in tree" :key="provider.id" class="mt-2 first:mt-0">
        <div class="px-3 py-1 text-xs font-semibold uppercase tracking-wide text-base-content/55">
          {{ provider.name }}
        </div>
        <div v-for="model in provider.models" :key="model.key" class="mb-1 rounded-lg bg-base-200/60 p-1">
          <div class="px-2 py-1.5 text-sm font-medium">
            <div class="truncate">{{ model.name }}</div>
            <div v-if="model.summaryFields.length > 0" class="mt-0.5 truncate text-xs font-normal text-base-content/60">
              {{ summary(model.representative, model.summaryFields) }}
            </div>
          </div>
          <button
            v-for="leaf in model.leaves"
            :key="leaf.id"
            type="button"
            class="flex w-full items-center rounded-md px-3 py-1.5 text-left text-sm hover:bg-base-100"
            :class="leaf.id === modelValue ? 'bg-primary/10 font-medium text-primary' : ''"
            @click="selectValue(leaf.id)"
          >
            {{ leaf.label }}
          </button>
        </div>
      </section>
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
  type ApiConfigSelectionModel,
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
const summaryLabels = computed<Record<ApiConfigSelectionSummaryField, string>>(() => ({
  contextWindowTokens: t("config.api.contextWindow"),
  maxOutputTokens: t("config.api.maxOutputTokens"),
  temperature: t("config.api.temperature"),
  enableTools: t("config.api.capTools"),
  enableImage: t("config.api.capImage"),
  enableAudio: t("config.api.capAudio"),
  enableVideo: t("config.api.capVideo"),
}));

function summary(item: ApiConfigItem, fields: ApiConfigSelectionModel["summaryFields"]): string {
  return apiConfigSelectionSummary(item, fields, summaryLabels.value);
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
