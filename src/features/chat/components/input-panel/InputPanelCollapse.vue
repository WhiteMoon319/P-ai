<template>
  <!-- 高度折叠壳：整区显隐走网格高度动画，内容增删走条目过渡 -->
  <div
    class="grid transition-all duration-300 ease-in-out"
    :class="expanded ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'"
  >
    <div class="min-h-0 overflow-hidden">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    expanded?: boolean;
  }>(),
  {
    expanded: true,
  },
);
</script>
