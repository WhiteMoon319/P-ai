<template>
  <ConfigTemplate :model-value="templateValues" :groups="templateGroups">
    <template #group-actions-notification>
      <button
        class="btn btn-sm btn-primary shrink-0"
        :disabled="!notificationDirty || props.savingConfig"
        @click="handleSaveConfig"
      >
        {{ props.savingConfig ? t("common.saving") : t("common.save") }}
      </button>
    </template>
    <template #row-enable-notification>
      <label class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.notification.enableLabel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.enableHint") }}</div>
        </div>
        <input
          :checked="props.config.messageNotificationEnabled"
          class="toggle toggle-sm toggle-primary shrink-0"
          type="checkbox"
          @change="props.config.messageNotificationEnabled = ($event.target as HTMLInputElement).checked"
        />
      </label>
    </template>
    <template #row-sound-notification>
      <label
        class="flex min-w-0 items-center justify-between gap-4"
        :class="{ 'opacity-50': !props.config.messageNotificationEnabled }"
      >
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.notification.soundLabel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.soundHint") }}</div>
        </div>
        <input
          :checked="props.config.messageNotificationSoundEnabled"
          :disabled="!props.config.messageNotificationEnabled"
          class="toggle toggle-sm toggle-primary shrink-0"
          type="checkbox"
          @change="props.config.messageNotificationSoundEnabled = ($event.target as HTMLInputElement).checked"
        />
      </label>
    </template>
    <template #row-test-notification>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.notification.testLabel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.testHint") }}</div>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <button
            class="btn btn-sm btn-outline"
            :disabled="testingNormal || testingLiveUpdate"
            @click="handleTestNotification('normal')"
          >
            {{ testingNormal ? t("common.loading") : t("config.notification.testNormalLabel") }}
          </button>
          <button
            class="btn btn-sm btn-outline"
            :disabled="testingNormal || testingLiveUpdate"
            @click="handleTestNotification('live_update')"
          >
            {{ testingLiveUpdate ? t("common.loading") : t("config.notification.testLiveUpdateLabel") }}
          </button>
        </div>
      </div>
    </template>
  </ConfigTemplate>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import type { ConfigTemplateGroup } from "../../components/config-template";
import type { AppConfig } from "../../../../types/app";
import { sendTransportNotificationTest } from "../../../../services/tauri-api";

const props = defineProps<{
  config: AppConfig;
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  lastSavedConfigJson: string;
}>();

const { t } = useI18n();
const templateValues = {};
const templateGroups = computed<ConfigTemplateGroup[]>(() => [
  {
    key: "notification",
    title: t("config.notification.title"),
    rows: [
      { key: "enable-notification", items: [] },
      { key: "sound-notification", items: [] },
      { key: "test-notification", items: [] },
    ],
  },
]);

const testingNormal = ref(false);
const testingLiveUpdate = ref(false);

async function handleTestNotification(kind: "normal" | "live_update") {
  if (kind === "normal") {
    testingNormal.value = true;
  } else {
    testingLiveUpdate.value = true;
  }
  try {
    await sendTransportNotificationTest<{ kind: string; sentAt: string; title: string; body: string }>(kind);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    // 通知测试失败时提示原因（如权限被拒），成功后系统通知本身就是反馈。
    console.warn("[通知测试] 失败", message);
    alert(`${t("config.notification.testFailedLabel")}\n${message}`);
  } finally {
    testingNormal.value = false;
    testingLiveUpdate.value = false;
  }
}

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
