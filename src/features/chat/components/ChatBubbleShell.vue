<template>
  <div :class="['ecall-chat-bubble-shell', `ecall-chat-bubble-shell-${side}`, `ecall-chat-bubble-tone-${tone}`, { 'ecall-chat-bubble-separated': separated, 'ecall-chat-bubble-wide': wide }]">
    <template v-if="tone === 'user'">
      <div class="ecall-chat-bubble-user-row">
        <div class="ecall-chat-bubble-avatar" :title="name">
          <img v-if="avatarUrl" :src="avatarUrl" :alt="name" />
          <span v-else>{{ avatarLabel }}</span>
        </div>

        <div class="ecall-chat-bubble-body">
          <div class="ecall-chat-bubble-surface" :style="surfaceStyle">
            <slot />
          </div>

          <div v-if="$slots.footer" class="ecall-chat-bubble-footer">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="ecall-chat-bubble-head">
        <div class="ecall-chat-bubble-avatar" :title="name">
          <img v-if="avatarUrl" :src="avatarUrl" :alt="name" />
          <span v-else>{{ avatarLabel }}</span>
        </div>

        <div class="ecall-chat-bubble-main">
          <div class="ecall-chat-bubble-header">
            <span class="ecall-chat-bubble-name">{{ name }}</span>
            <span v-if="streaming" class="ecall-chat-bubble-meta ecall-chat-bubble-streaming-meta">
              <span class="loading loading-spinner loading-xs" aria-hidden="true"></span>
              {{ streamingText || "正在生成" }}
            </span>
            <span v-else-if="meta" class="ecall-chat-bubble-meta">{{ meta }}</span>
          </div>

          <div v-if="$slots.activity" class="ecall-chat-bubble-activity">
            <slot name="activity" />
          </div>
        </div>
      </div>

      <div class="ecall-chat-bubble-body">
        <div class="ecall-chat-bubble-surface" :style="surfaceStyle">
          <slot />
        </div>

        <div v-if="$slots.footer" class="ecall-chat-bubble-footer">
          <slot name="footer" />
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, type StyleValue } from "vue";

const props = withDefaults(defineProps<{
  side?: "left" | "right";
  tone?: "assistant" | "user" | "system";
  name: string;
  meta?: string;
  avatarText?: string;
  avatarUrl?: string;
  streaming?: boolean;
  streamingText?: string;
  separated?: boolean;
  wide?: boolean;
  bubbleBackground?: boolean;
}>(), {
  side: "left",
  tone: "assistant",
  meta: "",
  avatarText: "",
  avatarUrl: "",
  streaming: false,
  streamingText: "",
  separated: false,
  wide: false,
  bubbleBackground: false,
});

const avatarLabel = computed(() => {
  const explicit = String(props.avatarText || "").trim();
  if (explicit) return explicit.slice(0, 2).toUpperCase();
  const name = String(props.name || "").trim();
  return (name ? name.slice(0, 1) : "?").toUpperCase();
});

const surfaceStyle = computed<StyleValue | undefined>(() => {
  if (props.tone === "user") {
    return {
      borderRadius: "var(--radius-box, 1rem)",
      backgroundColor: "var(--color-base-300)",
      padding: "0.68rem 0.82rem",
    };
  }
  if (!props.bubbleBackground) return undefined;
  return {
    borderRadius: "var(--radius-box, 1rem)",
    backgroundColor: "var(--color-base-100)",
    padding: "0.68rem 0.82rem",
  };
});
</script>

<style scoped>
.ecall-chat-bubble-shell {
  --ecall-bubble-avatar-size: 2rem;
  --ecall-bubble-gap: 0.55rem;
  --ecall-bubble-max-width: 42rem;
  --ecall-bubble-avatar-track: calc(var(--ecall-bubble-avatar-size) + var(--ecall-bubble-gap));
  --ecall-bubble-body-offset: calc(var(--ecall-bubble-avatar-size) / 2);
  position: relative;
  width: 100%;
}

.ecall-chat-bubble-wide {
  --ecall-bubble-max-width: 100%;
}

.ecall-chat-bubble-separated::before {
  position: absolute;
  top: -0.5rem;
  left: var(--ecall-bubble-body-offset);
  width: min(var(--ecall-bubble-max-width), calc(100% - var(--ecall-bubble-body-offset)));
  height: 1px;
  background: color-mix(in srgb, var(--color-base-content) 14%, transparent);
  content: "";
  pointer-events: none;
  transform: scaleY(0.5);
  transform-origin: center;
}

.ecall-chat-bubble-shell-right.ecall-chat-bubble-separated::before {
  right: var(--ecall-bubble-body-offset);
  left: auto;
}

.ecall-chat-bubble-head,
.ecall-chat-bubble-user-row {
  display: flex;
  max-width: min(100%, calc(var(--ecall-bubble-max-width) + var(--ecall-bubble-avatar-track)));
  align-items: flex-start;
  gap: var(--ecall-bubble-gap);
}

.ecall-chat-bubble-body {
  display: flex;
  width: min(var(--ecall-bubble-max-width), calc(100% - var(--ecall-bubble-body-offset)));
  max-width: calc(100% - var(--ecall-bubble-body-offset));
  min-width: 0;
  margin-left: var(--ecall-bubble-body-offset);
  flex-direction: column;
  gap: 0.25rem;
}

.ecall-chat-bubble-shell-right .ecall-chat-bubble-head,
.ecall-chat-bubble-shell-right .ecall-chat-bubble-user-row {
  margin-left: auto;
  flex-direction: row-reverse;
}

.ecall-chat-bubble-shell-right .ecall-chat-bubble-body {
  margin-right: var(--ecall-bubble-body-offset);
  margin-left: auto;
  align-items: flex-end;
}

.ecall-chat-bubble-tone-user .ecall-chat-bubble-body {
  width: auto;
  max-width: min(var(--ecall-bubble-max-width), calc(100% - var(--ecall-bubble-avatar-track) - var(--ecall-bubble-body-offset)));
  margin: 0;
  flex: 0 1 auto;
}

.ecall-chat-bubble-shell:not(.ecall-chat-bubble-tone-user) {
  display: grid;
  grid-template-columns: var(--ecall-bubble-avatar-size) minmax(0, min(var(--ecall-bubble-max-width), calc(100% - var(--ecall-bubble-avatar-track) - var(--ecall-bubble-body-offset))));
  grid-template-areas:
    "avatar main"
    "body body";
  column-gap: var(--ecall-bubble-gap);
  align-items: start;
}

.ecall-chat-bubble-shell-right:not(.ecall-chat-bubble-tone-user) {
  grid-template-columns: minmax(0, min(var(--ecall-bubble-max-width), calc(100% - var(--ecall-bubble-avatar-track) - var(--ecall-bubble-body-offset)))) var(--ecall-bubble-avatar-size);
  grid-template-areas:
    "main avatar"
    "body body";
}

.ecall-chat-bubble-shell:not(.ecall-chat-bubble-tone-user) .ecall-chat-bubble-head {
  display: contents;
}

.ecall-chat-bubble-shell:not(.ecall-chat-bubble-tone-user) .ecall-chat-bubble-avatar {
  grid-area: avatar;
}

.ecall-chat-bubble-shell:not(.ecall-chat-bubble-tone-user) .ecall-chat-bubble-main {
  grid-area: main;
}

.ecall-chat-bubble-shell:not(.ecall-chat-bubble-tone-user) .ecall-chat-bubble-body {
  box-sizing: border-box;
  grid-area: body;
  padding-inline: var(--ecall-bubble-body-offset);
  width: min(var(--ecall-bubble-max-width), 100%);
  max-width: 100%;
  margin: 0;
}

.ecall-chat-bubble-shell-right:not(.ecall-chat-bubble-tone-user) .ecall-chat-bubble-body {
  justify-self: end;
}

.ecall-chat-bubble-avatar {
  display: inline-flex;
  flex: 0 0 var(--ecall-bubble-avatar-size);
  width: var(--ecall-bubble-avatar-size);
  height: var(--ecall-bubble-avatar-size);
  margin-top: 0.18rem;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 999px;
  background: var(--color-neutral);
  color: var(--color-neutral-content);
  font-size: 0.86rem;
  font-weight: 650;
  line-height: 1;
}

.ecall-chat-bubble-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.ecall-chat-bubble-main {
  display: flex;
  min-width: 0;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.2rem;
}

.ecall-chat-bubble-shell-right .ecall-chat-bubble-main {
  align-items: flex-end;
}

.ecall-chat-bubble-header,
.ecall-chat-bubble-footer {
  display: inline-flex;
  max-width: 100%;
  align-items: baseline;
  gap: 0.45rem;
}

.ecall-chat-bubble-shell-right .ecall-chat-bubble-footer {
  flex-direction: row-reverse;
}

.ecall-chat-bubble-shell-right .ecall-chat-bubble-header {
  flex-direction: row-reverse;
  align-items: flex-end;
}

.ecall-chat-bubble-name {
  min-width: 0;
  overflow: hidden;
  color: color-mix(in srgb, var(--color-base-content) 86%, transparent);
  font-size: 0.78rem;
  font-weight: 560;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ecall-chat-bubble-meta {
  color: color-mix(in srgb, var(--color-base-content) 55%, transparent);
  font-size: 0.78rem;
  line-height: 1.2;
}

.ecall-chat-bubble-footer {
  color: color-mix(in srgb, var(--color-base-content) 42%, transparent);
  font-size: 0.78rem;
  line-height: 1.2;
}

.ecall-chat-bubble-streaming-meta {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  color: var(--color-primary);
  font-size: 0.72rem;
}

.ecall-chat-bubble-streaming-meta .loading {
  width: 0.62rem;
  height: 0.62rem;
}

.ecall-chat-bubble-surface {
  width: fit-content;
  max-width: 100%;
  color: var(--color-base-content);
}

.ecall-chat-bubble-activity {
  width: min(100%, 36rem);
}

.ecall-chat-bubble-tone-assistant .ecall-chat-bubble-surface,
.ecall-chat-bubble-tone-system .ecall-chat-bubble-surface {
  padding: 0.15rem 0;
}

.ecall-chat-bubble-footer {
  min-height: 1.25rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 120ms ease;
}

.ecall-chat-bubble-shell:hover .ecall-chat-bubble-footer,
.ecall-chat-bubble-shell:focus-within .ecall-chat-bubble-footer {
  opacity: 1;
  pointer-events: auto;
}
</style>
