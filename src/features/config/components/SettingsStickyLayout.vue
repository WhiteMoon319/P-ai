<template>
  <div class="relative flex h-full min-h-0 flex-col gap-3">
    <div v-if="$slots.header" class="shrink-0">
      <div class="w-full px-4 pt-4">
        <slot name="header" />
      </div>
    </div>

    <div class="scrollbar-gutter-stable min-h-0 flex-1 overflow-y-auto overflow-x-hidden pb-24">
      <div class="w-full p-4">
        <slot />
      </div>
    </div>
  </div>
</template>

<style scoped>
.scrollbar-gutter-stable {
  scrollbar-gutter: stable;
}
</style>
