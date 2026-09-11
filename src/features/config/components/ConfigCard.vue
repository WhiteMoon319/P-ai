<template>
  <div class="overflow-hidden rounded-box border border-base-300 bg-base-100">
    <header
      v-if="title"
      class="flex items-center justify-between gap-3 border-b-2 border-base-200 px-4 py-3"
    >
      <h3 class="text-sm font-semibold">{{ title }}</h3>
      <div class="flex items-center gap-2">
        <slot name="actions" />
      </div>
    </header>
    <div :class="flush ? '' : 'px-3'">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  title?: string;
  flush?: boolean;
}>(), {
  flush: false,
});
</script>
