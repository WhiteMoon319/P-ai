<template>
  <dialog class="modal ecall-image-preview-modal" :class="{ 'modal-open': open }" @cancel.prevent="emit('close')">
    <div class="modal-box relative flex h-full max-h-full w-screen max-w-none flex-col overflow-hidden rounded-none bg-black/95 p-0 text-white shadow-none">
      <div class="absolute bottom-4 left-1/2 z-10 flex max-w-[calc(100vw-2rem)] -translate-x-1/2 items-center gap-0.5 rounded-lg border border-white/10 bg-black/45 p-1">
        <button class="btn btn-sm btn-ghost border-0 text-white/80 shadow-none hover:bg-white/10 hover:text-white" :disabled="zoom <= minZoom" @click="emit('zoomOut')">
          <Minus class="h-4 w-4" />
        </button>
        <button class="btn btn-sm btn-ghost border-0 text-white/80 shadow-none hover:bg-white/10 hover:text-white" :disabled="zoom >= maxZoom" @click="emit('zoomIn')">
          <Plus class="h-4 w-4" />
        </button>
        <button class="btn btn-sm min-w-14 border-0 bg-transparent text-white/80 shadow-none hover:bg-white/10 hover:text-white" :disabled="Math.abs(zoom - 1) < 0.001" @click="emit('reset')">
          {{ Math.round(zoom * 100) }}%
        </button>
        <template v-if="localPath">
          <button class="btn btn-sm btn-ghost border-0 text-white/80 shadow-none hover:bg-white/10 hover:text-white" :disabled="copyStatus === 'doing'" @click="emit('copyImage', localPath)">
            <Copy class="h-4 w-4" />
          </button>
          <button class="btn btn-sm btn-ghost border-0 text-white/80 shadow-none hover:bg-white/10 hover:text-white" :disabled="saveStatus === 'doing'" @click="emit('saveImage', localPath)">
            <Download class="h-4 w-4" />
          </button>
        </template>
        <span class="mx-1 h-4 w-px shrink-0 bg-white/15" aria-hidden="true"></span>
        <button class="btn btn-sm btn-ghost btn-square border-0 text-white/80 shadow-none hover:bg-white/10 hover:text-white" type="button" title="返回" aria-label="返回" @click="emit('close')">
          <ArrowLeft class="h-4 w-4" />
        </button>
      </div>
      <div
        class="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-0"
        :class="zoom > 1 ? (dragging ? 'cursor-grabbing' : 'cursor-grab') : ''"
        @wheel.prevent="emit('wheel', $event)"
        @pointermove="emit('pointerMove', $event)"
        @pointerup="emit('pointerUp', $event)"
        @pointercancel="emit('pointerUp', $event)"
        @pointerleave="emit('pointerUp', $event)"
      >
        <img
          v-if="dataUrl"
          :src="dataUrl"
          class="max-h-full max-w-full object-contain rounded select-none"
          draggable="false"
          :style="{ transform: `translate(${offsetX}px, ${offsetY}px) scale(${zoom})`, transformOrigin: 'center center', touchAction: 'none' }"
          @dragstart.prevent
          @pointerdown="emit('pointerDown', $event)"
        />
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { ArrowLeft, Copy, Download, Minus, Plus } from "@lucide/vue";

defineProps<{
  open: boolean;
  dataUrl: string;
  zoom: number;
  minZoom: number;
  maxZoom: number;
  offsetX: number;
  offsetY: number;
  dragging: boolean;
  localPath?: string;
  copyStatus?: "idle" | "doing";
  saveStatus?: "idle" | "doing";
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "zoomIn"): void;
  (e: "zoomOut"): void;
  (e: "reset"): void;
  (e: "wheel", event: WheelEvent): void;
  (e: "pointerDown", event: PointerEvent): void;
  (e: "pointerMove", event: PointerEvent): void;
  (e: "pointerUp", event: PointerEvent): void;
  (e: "copyImage", path: string): void;
  (e: "saveImage", path: string): void;
}>();
</script>

<style>
.ecall-image-preview-modal {
  top: 2.5rem;
  height: calc(100vh - 2.5rem);
}
</style>
