<template>
  <ul
    ref="rootRef"
    class="menu menu-sm w-full p-1"
    @contextmenu.prevent="collapseAllProviders"
  >
    <li v-if="placeholder">
      <button
        type="button"
        class="text-sm"
        :class="!selectedId ? 'active' : ''"
        @click="emitSelect('')"
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
          :class="option.id === selectedId ? 'badge-primary bg-primary/10' : ''"
          @click="emitSelect(option.id)"
        >
          {{ option.label }}
        </button>
      </div>
    </li>
    <!-- 供应商层始终保留；只有模型单档位时省略思维子菜单 -->
    <template v-for="provider in tree" :key="provider.id">
      <li>
        <details :open="isCurrentProvider(provider)">
          <summary class="font-semibold text-sm">
            <span class="min-w-0 flex-1 truncate">{{ provider.name }}</span>
            <span class="badge badge-ghost badge-xs shrink-0">{{ modelCount(provider) }}</span>
          </summary>
          <ul>
            <template v-for="model in provider.models" :key="model.key">
              <li v-if="model.leaves.length === 1">
                <button
                  type="button"
                  class="text-sm"
                  :class="isSelected(model.leaves[0].id) ? 'active' : ''"
                  @click.stop="emitSelect(model.leaves[0].id)"
                >
                  <span class="min-w-0 flex-1 truncate">{{ model.name }}</span>
                  <span v-if="model.leaves[0].label" class="badge badge-ghost badge-xs shrink-0">
                    {{ model.leaves[0].label }}
                  </span>
                  <template v-if="model.summaryFields.length > 0">
                    <span
                      v-for="field in model.summaryFields"
                      :key="field"
                      class="badge badge-outline badge-xs font-mono shrink-0"
                    >
                      {{ summaryValue(model.representative, field) }}
                    </span>
                  </template>
                  <span v-if="isSelected(model.leaves[0].id)" class="badge badge-xs badge-primary shrink-0">✓</span>
                </button>
              </li>
              <li v-else>
                <details :open="isCurrentModel(model)">
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
                        :class="isSelected(leaf.id) ? 'active' : ''"
                        @click.stop="emitSelect(leaf.id)"
                      >
                        <span class="min-w-0 flex-1 truncate">{{ leaf.label }}</span>
                        <span v-if="isSelected(leaf.id)" class="badge badge-xs badge-primary shrink-0">✓</span>
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
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem } from "../../../types/app";
import {
  apiConfigSelectionSummary,
  type ApiConfigSelectionProvider,
  type ApiConfigSelectionSummaryField,
} from "../utils/api-config-selection-tree";

const props = withDefaults(defineProps<{
  tree: ApiConfigSelectionProvider[];
  selectedId?: string;
  extraOptions?: Array<{ id: string; label: string }>;
  placeholder?: string;
}>(), {
  selectedId: "",
  extraOptions: () => [],
  placeholder: "",
});

const emit = defineEmits<{
  (event: "select", value: string): void;
}>();

const { t } = useI18n();
const rootRef = ref<HTMLElement | null>(null);

const summaryFieldLabels = computed<Record<ApiConfigSelectionSummaryField, string>>(() => ({
  contextWindowTokens: t("config.api.contextWindow"),
  maxOutputTokens: t("config.api.maxOutputTokens"),
  temperature: t("config.api.temperature"),
  enableTools: t("config.api.capTools"),
  enableImage: t("config.api.capImage"),
  enableAudio: t("config.api.capAudio"),
  enableVideo: t("config.api.capVideo"),
}));

function isSelected(id: string): boolean {
  return id === props.selectedId;
}

function summaryValue(item: ApiConfigItem, field: ApiConfigSelectionSummaryField): string {
  return apiConfigSelectionSummary(item, [field], summaryFieldLabels.value);
}

function modelCount(provider: ApiConfigSelectionProvider): number {
  return provider.models.reduce((sum, model) => sum + model.leaves.length, 0);
}

function isCurrentModel(model: { leaves: Array<{ id: string }> }): boolean {
  if (!props.selectedId) return false;
  return model.leaves.some((leaf) => leaf.id === props.selectedId);
}

function isCurrentProvider(provider: ApiConfigSelectionProvider): boolean {
  if (!props.selectedId) return false;
  return provider.models.some((model) => model.leaves.some((leaf) => leaf.id === props.selectedId));
}

function collapseAllProviders() {
  const root = rootRef.value;
  if (!root) return;
  // 只收供应商层：直接挂在根 menu 下的 details
  for (const details of root.querySelectorAll<HTMLDetailsElement>(":scope > li > details")) {
    details.open = false;
  }
}

function emitSelect(value: string) {
  emit("select", value);
}
</script>
