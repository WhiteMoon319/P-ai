<template>
  <div class="grid gap-3">
    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-4 p-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h3 class="card-title text-base">{{ t("config.notification.title") }}</h3>
            <p class="mt-1 text-xs text-base-content/60">{{ t("config.notification.summary") }}</p>
          </div>
          <button
            class="btn btn-sm btn-primary shrink-0"
            :disabled="!notificationDirty || props.savingConfig"
            @click="handleSaveConfig"
          >
            {{ props.savingConfig ? t("common.saving") : t("common.save") }}
          </button>
        </div>

        <label class="flex items-center justify-between gap-4 rounded-box border border-base-300 bg-base-200/40 p-4">
          <div class="min-w-0">
            <div class="text-sm font-medium">{{ t("config.notification.enableLabel") }}</div>
            <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.enableHint") }}</div>
          </div>
          <input
            :checked="props.config.messageNotificationEnabled"
            class="toggle toggle-primary"
            type="checkbox"
            @change="props.config.messageNotificationEnabled = ($event.target as HTMLInputElement).checked"
          />
        </label>

        <label
          class="flex items-center justify-between gap-4 rounded-box border border-base-300 bg-base-200/40 p-4"
          :class="{ 'opacity-50': !props.config.messageNotificationEnabled }"
        >
          <div class="min-w-0">
            <div class="text-sm font-medium">{{ t("config.notification.soundLabel") }}</div>
            <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.soundHint") }}</div>
          </div>
          <input
            :checked="props.config.messageNotificationSoundEnabled"
            :disabled="!props.config.messageNotificationEnabled"
            class="toggle toggle-primary"
            type="checkbox"
            @change="props.config.messageNotificationSoundEnabled = ($event.target as HTMLInputElement).checked"
          />
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { AppConfig } from "../../../../types/app";

const props = defineProps<{
  config: AppConfig;
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  lastSavedConfigJson: string;
}>();

const { t } = useI18n();

const savedNotificationSnapshot = computed(() => {
  try {
    const parsed = JSON.parse(String(props.lastSavedConfigJson || "{}")) as Partial<AppConfig>;
    return {
      messageNotificationEnabled: parsed.messageNotificationEnabled !== false,
      messageNotificationSoundEnabled: parsed.messageNotificationSoundEnabled === true,
    };
  } catch {
    return {
      messageNotificationEnabled: true,
      messageNotificationSoundEnabled: false,
    };
  }
});

const notificationDirty = computed(() => (
  props.config.messageNotificationEnabled !== savedNotificationSnapshot.value.messageNotificationEnabled
  || props.config.messageNotificationSoundEnabled !== savedNotificationSnapshot.value.messageNotificationSoundEnabled
));

async function handleSaveConfig() {
  if (!notificationDirty.value) return;
  await Promise.resolve(props.saveConfigAction());
}
</script>
