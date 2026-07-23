<template>
  <dialog class="modal !items-start pt-[8vh]" :class="{ 'modal-open': open }">
    <div class="modal-box mx-auto max-w-md overflow-visible">
      <h3 class="text-base font-semibold">{{ t("chat.newConversation") }}</h3>
      <div class="mt-3 flex flex-col gap-3">
        <input
          v-model="localTitle"
          type="text"
          class="input input-bordered w-full"
          :placeholder="t('chat.newConversationTopicPlaceholder')"
          @keydown.enter.prevent="confirm"
        />
        <DepartmentPersonaSelect
          v-model:department-id="localDepartmentId"
          v-model:agent-id="localAgentId"
          :options="departments"
          :persona-avatar-url-map="personaAvatarUrlMap"
          auto-select-first
        />
        <div class="text-xs text-base-content/60">{{ t("chat.createConversationDepartmentPersonaLockedHint") }}</div>
        <select
          v-model="localWorkMode"
          class="select select-bordered w-full"
          :disabled="!workspacePath"
        >
          <option value="directory">{{ t("chat.workspaceWorkModeDirectory") }}</option>
          <option value="isolated_worktree" :disabled="workspaceAccess === 'read_only' || !worktreeAvailable">{{ t("chat.workspaceWorkModeIsolated") }}</option>
        </select>
        <div
          v-if="workspacePath && workspaceAccess !== 'read_only' && worktreeCheckMessage"
          class="text-xs text-base-content/60"
        >
          {{ worktreeCheckMessage }}
        </div>
      </div>
      <div v-if="errorText" class="mt-3 rounded border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" :disabled="creating" @click="emit('close')">{{ t("common.cancel") }}</button>
        <button class="btn btn-sm btn-primary" :disabled="creating || !localDepartmentId || !localAgentId" @click="confirm">
          <span v-if="creating" class="loading loading-spinner loading-xs"></span>
          <span>{{ creating ? t("chat.createConversationCreating") : t("chat.createConversationAction") }}</span>
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ShellWorkspaceAccess, ShellWorkMode } from "../../../types/app";
import DepartmentPersonaSelect from "../../shared/components/DepartmentPersonaSelect.vue";
import type { SidebarCreateDepartmentOption } from "../sidebar-app-types";

const { t } = useI18n();

const props = defineProps<{
  open: boolean;
  creating: boolean;
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
  workspacePath: string;
  workspaceAccess: ShellWorkspaceAccess;
  worktreeAvailable: boolean;
  worktreeCheckMessage: string;
  personaAvatarUrlMap?: Record<string, string>;
  errorText: string;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [input: { title?: string; departmentId: string; agentId: string; shellWorkMode: ShellWorkMode }];
}>();

const localTitle = ref("");
const localDepartmentId = ref("");
const localAgentId = ref("");
const localWorkMode = ref<ShellWorkMode>("directory");

watch(
  () => [props.open, props.defaultDepartmentId, props.departments.map((item) => item.id).join("|")] as const,
  ([open]) => {
    if (!open) return;
    localTitle.value = "";
    const option = props.departments.find((item) =>
      String(item.departmentId || "").trim() === String(props.defaultDepartmentId || "").trim()
    ) || props.departments[0];
    localDepartmentId.value = String(option?.departmentId || props.defaultDepartmentId || "").trim();
    localAgentId.value = String(option?.agentId || "").trim();
    localWorkMode.value = "directory";
  },
  { immediate: true },
);

watch(
  () => [props.workspaceAccess, props.worktreeAvailable] as const,
  ([access, worktreeAvailable]) => {
    if (access === "read_only" || !worktreeAvailable) {
      localWorkMode.value = "directory";
    }
  },
);

function confirm() {
  const departmentId = String(localDepartmentId.value || "").trim();
  const agentId = String(localAgentId.value || "").trim();
  if (!departmentId || !agentId) return;
  emit("confirm", {
    title: String(localTitle.value || "").trim() || undefined,
    departmentId,
    agentId,
    shellWorkMode: props.workspaceAccess === "read_only"
      || !props.worktreeAvailable
      || !String(props.workspacePath || "").trim()
      ? "directory"
      : localWorkMode.value,
  });
}
</script>
