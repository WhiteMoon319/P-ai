<template>
  <div class="grid gap-7">
    <section v-for="group in groups" :key="group.title">
      <h3 class="mb-3 text-base font-semibold">{{ group.title }}</h3>
      <div class="divide-y divide-base-200 overflow-hidden rounded-box bg-base-100">
          <div
            v-for="(row, rowIndex) in group.rows"
            :key="`${group.title}-${rowIndex}`"
            class="grid gap-4 px-5 py-4"
            :class="row.items.length > 1 ? 'sm:grid-cols-2' : 'grid-cols-1'"
          >
            <template v-for="field in row.items" :key="field.key">
            <div
              v-if="field.type === 'toggle' || field.type === 'select'"
              class="flex min-h-10 min-w-0 items-center justify-between gap-4"
            >
              <div class="min-w-0">
                <div class="text-sm">{{ field.label }}</div>
                <p v-if="field.description" class="mt-1 text-xs text-base-content/60">{{ field.description }}</p>
              </div>
              <input
                v-if="field.type === 'toggle'"
                :checked="Boolean(fieldValue(field.key))"
                :disabled="field.disabled"
                type="checkbox"
                class="toggle toggle-primary shrink-0"
                @change="updateField(field.key, ($event.target as HTMLInputElement).checked)"
              />
              <select
                v-else
                :value="String(fieldValue(field.key) ?? '')"
                :disabled="field.disabled"
                class="select select-bordered select-sm w-52 max-w-full shrink-0"
                @change="updateField(field.key, ($event.target as HTMLSelectElement).value)"
              >
                <option v-for="option in field.options || []" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </div>

            <label v-else class="grid min-w-0 gap-2">
              <div>
                <div class="text-sm">{{ field.label }}</div>
                <div v-if="field.description" class="mt-1 text-xs text-base-content/60">{{ field.description }}</div>
              </div>
              <input
                v-if="field.type === 'text' || field.type === 'number'"
                :value="String(fieldValue(field.key) ?? '')"
                :placeholder="field.placeholder"
                :disabled="field.disabled"
                :type="field.type"
                :min="field.min"
                :max="field.max"
                :step="field.step"
                class="input input-bordered input-sm w-full"
                @input="updateField(field.key, field.type === 'number'
                  ? Number(($event.target as HTMLInputElement).value)
                  : ($event.target as HTMLInputElement).value)"
              />
              <textarea
                v-else
                :value="String(fieldValue(field.key) ?? '')"
                :placeholder="field.placeholder"
                :disabled="field.disabled"
                class="textarea textarea-bordered textarea-sm min-h-24 w-full"
                @input="updateField(field.key, ($event.target as HTMLTextAreaElement).value)"
              />
            </label>
            </template>
          </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import type { ConfigTemplateGroup } from "./config-template";

const props = defineProps<{
  modelValue: Record<string, unknown>;
  groups: ConfigTemplateGroup[];
}>();
const emit = defineEmits<{
  "update:modelValue": [value: Record<string, unknown>];
}>();

function fieldValue(key: string): unknown {
  return props.modelValue[key];
}

function updateField(key: string, value: unknown) {
  emit("update:modelValue", { ...props.modelValue, [key]: value });
}
</script>
