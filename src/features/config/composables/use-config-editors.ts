import type { ComputedRef, Ref } from "vue";
import type { ApiConfigItem, ApiProviderConfigItem, AppConfig, PersonaProfile } from "../../../types/app";
import { defaultToolBindings } from "../utils/builtin-tools";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

type UseConfigEditorsOptions = {
  t: TrFn;
  config: AppConfig;
  personas: Ref<PersonaProfile[]>;
  assistantPersonas: ComputedRef<PersonaProfile[]>;
  assistantDepartmentAgentId: Ref<string>;
  personaEditorId: Ref<string>;
  selectedPersonaEditor: ComputedRef<PersonaProfile | null>;
  createApiConfig: (seed?: string) => ApiConfigItem;
  createApiProvider: (seed?: string) => ApiProviderConfigItem;
  normalizeApiBindingsLocal: () => void;
  savePersonas: () => Promise<boolean>;
  saveChatPreferences: () => Promise<void>;
  saveConfig: () => Promise<boolean>;
};

export type PersonaDepartmentToggleStatus =
  | "applied"
  | "unchanged"
  | "overridden"
  | "failed"
  | "rejected";

export type PersonaDepartmentToggleResult = { status: PersonaDepartmentToggleStatus };

export function useConfigEditors(options: UseConfigEditorsOptions) {
  function firstActiveApiConfigId(): string {
    for (const provider of options.config.apiProviders || []) {
      if (provider.deprecated) continue;
      for (const model of provider.models || []) {
        if (model.deprecated) continue;
        const providerId = String(provider.id || "").trim();
        const modelId = String(model.id || "").trim();
        if (providerId && modelId) return `${providerId}::${modelId}`;
      }
    }
    return "";
  }

  function addApiConfig() {
    const provider = options.createApiProvider();
    options.config.apiProviders.push(provider);
    options.normalizeApiBindingsLocal();
    options.config.selectedApiConfigId = `${provider.id}::${provider.models[0]?.id || ""}`;
  }

  function removeSelectedApiConfig() {
    const [providerId, modelId] = String(options.config.selectedApiConfigId || "").split("::");
    if (!providerId) return;
    const providerIdx = options.config.apiProviders.findIndex((item) => item.id === providerId);
    if (providerIdx < 0) return;
    const provider = options.config.apiProviders[providerIdx];
    const removedId = String(options.config.selectedApiConfigId || "").trim();
    const activeProviders = (options.config.apiProviders || []).filter((item) => !item.deprecated);
    const activeModels = (provider.models || []).filter((item) => !item.deprecated);
    if (!provider.deprecated && activeProviders.length <= 1 && activeModels.length <= 1) return;
    if (modelId) {
      const model = (provider.models || []).find((item) => item.id === modelId);
      if (!model) return;
      if (!provider.deprecated && activeModels.length <= 1) {
        provider.deprecated = true;
        provider.models = (provider.models || []).map((item) => ({ ...item, deprecated: true }));
      } else {
        model.deprecated = true;
      }
    } else {
      provider.deprecated = true;
      provider.models = (provider.models || []).map((item) => ({ ...item, deprecated: true }));
    }
    for (const department of options.config.departments || []) {
      const nextIds = (Array.isArray(department.apiConfigIds) ? department.apiConfigIds : [])
        .map((id) => String(id || "").trim())
        .filter((id) => !!id && id !== removedId);
      department.apiConfigIds = nextIds;
      if (String(department.apiConfigId || "").trim() === removedId) {
        department.apiConfigId = nextIds[0] || "";
      }
    }
    if (options.config.assistantDepartmentApiConfigId === removedId) {
      options.config.assistantDepartmentApiConfigId = "";
    }
    if (options.config.sttApiConfigId === removedId) {
      options.config.sttApiConfigId = undefined;
      options.config.sttAutoSend = false;
    }
    if (options.config.visionApiConfigId === removedId) {
      options.config.visionApiConfigId = undefined;
    }
    if (options.config.toolReviewApiConfigId === removedId) {
      options.config.toolReviewApiConfigId = undefined;
    }
    options.normalizeApiBindingsLocal();
    options.config.selectedApiConfigId = firstActiveApiConfigId();
  }

  async function addPersona() {
    const previousPersonas = options.personas.value.map((persona) => ({
      ...persona,
      tools: Array.isArray(persona.tools)
        ? persona.tools.map((tool) => ({
            ...tool,
            args: Array.isArray(tool.args) ? [...tool.args] : [],
            values: { ...((tool.values || {}) as Record<string, unknown>) },
          }))
        : [],
    }));
    const previousAssistantDepartmentAgentId = options.assistantDepartmentAgentId.value;
    const previousPersonaEditorId = options.personaEditorId.value;
    const id = `persona-${Date.now()}`;
    const now = new Date().toISOString();
    options.personas.value.push({
      id,
      name: `${options.t("config.persona.title")} ${options.assistantPersonas.value.length + 1}`,
      systemPrompt: options.t("config.persona.assistantPlaceholder"),
      tools: defaultToolBindings(),
      privateMemoryEnabled: false,
      memoryRecallMode: "auto",
      createdAt: now,
      updatedAt: now,
      avatarPath: undefined,
      avatarUpdatedAt: undefined,
      isBuiltInUser: false,
      isBuiltInSystem: false,
      source: "main_config",
      scope: "global",
    });
    options.assistantDepartmentAgentId.value = id;
    options.personaEditorId.value = id;
    const saved = await options.savePersonas();
    if (!saved) {
      options.personas.value = previousPersonas;
      options.assistantDepartmentAgentId.value = previousAssistantDepartmentAgentId;
      options.personaEditorId.value = previousPersonaEditorId;
      return;
    }
    await options.saveChatPreferences();
  }

  /**
   * 勾选/取消人格与部门的归属关系，立即落盘。
   * 私域部门由私有工作区文件维护，不在这里改。
   *
   * 返回结构化结果：后端在保存时会自修复（内置部门缺人格回默认人格），
   * 那种情况下用户请求的状态不会落地，调用方必须能区分，不能一律当成已生效。
   */
  async function togglePersonaDepartmentMember(input: {
    agentId: string;
    departmentId: string;
    member: boolean;
  }): Promise<PersonaDepartmentToggleResult> {
    const agentId = String(input?.agentId || "").trim();
    const departmentId = String(input?.departmentId || "").trim();
    if (!agentId || !departmentId) return { status: "rejected" };
    const department = (options.config.departments || []).find(
      (item) => String(item.id || "").trim() === departmentId,
    );
    if (!department || String(department.source || "").trim() === "private_workspace") {
      return { status: "rejected" };
    }
    const previousAgentIds = Array.isArray(department.agentIds) ? [...department.agentIds] : [];
    const isMember = previousAgentIds.some((id) => String(id || "").trim() === agentId);
    if (isMember === !!input.member) return { status: "unchanged" };
    department.agentIds = input.member
      ? [...previousAgentIds, agentId]
      : previousAgentIds.filter((id) => String(id || "").trim() !== agentId);
    const saved = await options.saveConfig();
    if (!saved) {
      department.agentIds = previousAgentIds;
      return { status: "failed" };
    }
    // 保存会把归一化后的配置回写进来，据此确认请求的状态是否真的落地
    const savedDepartment = (options.config.departments || []).find(
      (item) => String(item.id || "").trim() === departmentId,
    );
    const appliedAgentIds = Array.isArray(savedDepartment?.agentIds) ? savedDepartment.agentIds : [];
    const applied = appliedAgentIds.some((id) => String(id || "").trim() === agentId);
    return { status: applied === !!input.member ? "applied" : "overridden" };
  }

  function removeSelectedPersona() {
    if (options.assistantPersonas.value.length <= 1) return;
    const target = options.selectedPersonaEditor.value;
    if (!target || target.isBuiltInUser || target.isBuiltInSystem) return;
    const idx = options.personas.value.findIndex((p) => p.id === target.id);
    if (idx >= 0) options.personas.value.splice(idx, 1);
    if (options.assistantDepartmentAgentId.value === target.id) {
      options.assistantDepartmentAgentId.value = options.assistantPersonas.value[0]?.id || "default-agent";
    }
    options.personaEditorId.value = options.assistantPersonas.value[0]?.id || "default-agent";
  }

  return {
    addApiConfig,
    removeSelectedApiConfig,
    addPersona,
    togglePersonaDepartmentMember,
    removeSelectedPersona,
  };
}

