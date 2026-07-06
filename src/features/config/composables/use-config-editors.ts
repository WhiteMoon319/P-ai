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
};

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
    removeSelectedPersona,
  };
}

