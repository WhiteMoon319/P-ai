<template>
  <div
    class="tabs tabs-box bg-base-200 p-1"
    :class="[
      fullWidth ? 'w-full' : 'inline-flex',
      disabled ? 'opacity-60' : '',
    ]"
  >
    <button
      v-for="option in options"
      :key="String(option.value)"
      type="button"
      class="tab rounded-btn"
      :class="[
        fullWidth ? 'flex-1' : '',
        size === 'sm' ? 'tab-sm' : '',
        isSelected(option.value) ? 'tab-active' : '',
      ]"
      :disabled="disabled || !!option.disabled"
      @click="selectValue(option.value)"
    >
      {{ option.label }}
    </button>
  </div>
</template>

<script setup lang="ts" generic="T extends string | number | boolean">
export type SegmentedControlOption<T extends string | number | boolean> = {
  value: T;
  label: string;
  disabled?: boolean;
};

const props = withDefaults(defineProps<{
  modelValue: T;
  options: Array<SegmentedControlOption<T>>;
  disabled?: boolean;
  fullWidth?: boolean;
  size?: "sm" | "md";
}>(), {
  disabled: false,
  fullWidth: true,
  size: "md",
});

const emit = defineEmits<{
  (e: "update:modelValue", value: T): void;
  (e: "change", value: T): void;
}>();

function isSelected(value: T): boolean {
  return props.modelValue === value;
}

function selectValue(value: T) {
  if (props.disabled || props.modelValue === value) return;
  emit("update:modelValue", value);
  emit("change", value);
}
</script>
