<template>
  <!-- 第二栏：输入卡内第二层，左右操作区，单行；吃卡的 p-2，不再外扩 -->
  <div>
    <div class="flex items-center justify-between gap-2">
      <div class="flex min-w-0 flex-1 items-center">
        <slot name="left" />
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <slot name="right" />
      </div>
    </div>
  </div>
</template>
