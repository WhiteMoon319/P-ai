<template>
  <div class="grid gap-3">
    <div class="flex items-center gap-2">
      <button class="btn btn-sm btn-square btn-primary shrink-0" type="button" :title="t('config.imageGeneration.addProvider')" @click="addProvider">
        <Plus class="h-4 w-4" />
      </button>
      <button class="btn btn-sm btn-square shrink-0" :class="providers.length <= 1 ? 'btn-disabled bg-base-200 text-base-content/30' : 'btn-error'" type="button" :title="t('config.imageGeneration.removeProvider')" :disabled="!selectedProvider || providers.length <= 1" @click="removeSelectedProvider">
        <Trash2 class="h-4 w-4" />
      </button>
      <select :value="selectedProviderId" class="select select-bordered select-md min-w-0 flex-1" :disabled="providers.length === 0" @change="selectedProviderId = ($event.target as HTMLSelectElement).value">
        <option v-if="providers.length === 0" value="">{{ t("config.imageGeneration.emptyProviders") }}</option>
        <option v-for="provider in providers" :key="provider.id" :value="provider.id">{{ provider.name || provider.id }}（{{ providerTypeLabel(provider.providerType) }}）</option>
      </select>
      <button class="btn btn-sm btn-square" :class="imageDirty ? 'btn-info' : 'bg-base-200 text-base-content/30 shadow-none'" type="button" :title="t('common.reset')" :disabled="!imageDirty || props.savingConfig" @click="restoreImageConfig"><RotateCcw class="h-4 w-4" /></button>
      <button class="api-save-btn btn btn-sm btn-square" :class="imageDirty ? 'btn-success api-save-btn--dirty' : 'bg-base-200 text-base-content/50 shadow-none'" type="button" :title="props.savingConfig ? t('common.saving') : imageDirty ? t('common.save') : t('common.saved')" :disabled="!imageDirty || props.savingConfig || !!enabledWorkflowError" @click="saveImageConfig"><Save v-if="!props.savingConfig" class="h-4 w-4" /><span v-else class="loading loading-spinner loading-sm" /></button>
    </div>

    <div v-if="selectedProvider" class="grid gap-3">
      <ConfigTemplate v-model="providerTemplateValues" :groups="providerTemplateGroups" />
      <section v-if="selectedProvider.providerType !== 'codex'">
        <h3 class="mb-1 text-base font-semibold">{{ t("config.imageGeneration.apiKeys") }}</h3>
        <p class="mb-3 text-xs text-base-content/60">{{ t("config.imageGeneration.apiKeysHint") }}</p>
        <div class="card border border-base-300 bg-base-100">
            <div class="card-body gap-3 p-4">
              <div class="flex items-center justify-between gap-2">
                <div class="text-sm font-medium">{{ t("config.api.apiKeyPool") }}</div>
                <button class="btn btn-sm bg-base-200" type="button" @click="addApiKey">
                  <Plus class="h-3.5 w-3.5" />
                  <span>{{ t("config.api.addApiKey") }}</span>
                </button>
              </div>

              <div class="grid gap-2">
                <div v-for="(apiKey, index) in selectedProvider.apiKeys" :key="`key-${selectedProvider.id}-${index}`"
                  class="flex items-center gap-2">
                  <span class="w-4 shrink-0" />
                  <input
                    v-model="selectedProvider.apiKeys[index]"
                    :type="showImageApiKeys[selectedProvider.id]?.[index] ? 'text' : 'password'"
                    class="input input-bordered input-sm flex-1"
                    :placeholder="`API Key #${index + 1}`"
                  />
                  <button
                    class="btn btn-sm btn-square bg-base-200"
                    type="button"
                    :disabled="index === 0"
                    :title="t('config.api.pinApiKeyToTop')"
                    @click="pinApiKeyToTop(index)"
                  >
                    <ArrowUpToLine class="h-3.5 w-3.5" />
                  </button>
                  <button
                    class="btn btn-sm btn-square bg-base-200"
                    type="button"
                    @click="toggleImageApiKeyVisible(selectedProvider.id, index)"
                  >
                    <EyeOff v-if="showImageApiKeys[selectedProvider.id]?.[index]" class="h-3.5 w-3.5" />
                    <Eye v-else class="h-3.5 w-3.5" />
                  </button>
                  <button
                    class="btn btn-sm btn-square bg-base-200 text-error"
                    type="button"
                    :disabled="selectedProvider.apiKeys.length <= 1"
                    @click="removeApiKey(index)"
                  >
                    <Trash2 class="h-3.5 w-3.5" />
                  </button>
                </div>
                <div
                  v-if="selectedProvider.apiKeys.length === 0"
                  class="rounded-box border border-dashed border-base-300 px-3 py-3 text-sm opacity-60"
                >
                  {{ t("config.api.noApiKey") }}
                </div>
              </div>
            </div>
        </div>
      </section>
      <div v-else class="rounded-box border border-info/30 bg-info/5 px-3 py-2 text-xs text-base-content/70">
        {{ t("config.imageGeneration.codexCredentialHint") }}
      </div>

      <section>
        <h3 class="mb-1 text-base font-semibold">{{ t("config.api.modelCards") }}</h3>
        <p class="mb-3 text-xs text-base-content/60">{{ t("config.api.modelCardsHint") }}</p>
        <div class="card border border-base-300 bg-base-100">
          <div class="card-body gap-3 p-4">
          <div class="flex items-start justify-between gap-3">
            <div />
            <button class="btn btn-sm btn-ghost" type="button" :title="t('config.imageGeneration.addModel')" @click="addModel">
              <Plus class="h-4 w-4" />
              <span>{{ t("config.imageGeneration.addModel") }}</span>
            </button>
          </div>

          <div v-if="selectedProvider.models.length" class="grid gap-3">
            <article v-for="model in selectedProvider.models" :key="model.id" class="rounded-box border border-base-300 bg-base-200/30 p-3">
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0 flex-1 truncate text-base font-semibold">{{ model.model || model.id }}</div>
                <button class="btn btn-square btn-ghost btn-sm text-error" type="button" :title="t('common.delete')" @click="removeModel(model.id)">
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
              <label class="mt-3 grid gap-1">
                <span class="text-sm font-medium">{{ t("config.api.model") }}</span>
                  <div class="join w-full">
                    <input class="input input-bordered input-sm join-item flex-1 font-mono" :value="model.model || model.id" readonly />
                    <button class="btn btn-sm join-item bg-base-300" type="button" @click="toggleImageModelPicker(model.id)"><ChevronDown class="h-3.5 w-3.5" /></button>
                  </div>
                  <div v-if="activeImageModelPickerId === model.id" class="rounded-box border border-base-300 bg-base-200/50 p-3">
                    <input v-model="imageModelSearch" class="input input-bordered input-sm mb-2 w-full" :placeholder="t('config.api.searchModel')" />
                    <div class="max-h-48 overflow-auto">
                      <button v-for="option in filteredImageModelOptions" :key="`${model.id}-${option}`" class="btn btn-ghost btn-sm mb-1 mr-1" type="button" @click="selectImageModel(model.id, option)">{{ option }}</button>
                      <div v-if="filteredImageModelOptions.length === 0" class="px-2 py-3 text-sm opacity-50">{{ t("config.api.noModelFound") }}</div>
                    </div>
                  </div>
              </label>
              <div class="mt-2 text-xs text-base-content/60">{{ t("config.api.matchedProtocol", { protocol: providerTypeLabel(selectedProvider.providerType) }) }}</div>
            </article>
          </div>
          <div v-else class="rounded-box border border-dashed border-base-300 p-4 text-center text-xs text-base-content/55">
            {{ t("config.imageGeneration.emptyModels") }}
          </div>

          </div>
        </div>
      </section>

      <section v-if="selectedProvider.providerType === 'comfyui'">
        <h3 class="mb-1 text-base font-semibold">{{ t("config.imageGeneration.comfyTitle") }}</h3>
        <p class="mb-3 text-xs text-base-content/60">{{ t("config.imageGeneration.comfyHint") }}</p>
        <div class="card border border-base-300 bg-base-100">
          <div class="card-body gap-4 p-4">
          <label class="grid gap-1">
            <span class="text-sm font-medium">{{ t("config.imageGeneration.workflowJson") }}</span>
            <textarea v-model="selectedProvider.comfyuiWorkflowJson" class="textarea textarea-bordered min-h-64 font-mono text-[11px] leading-relaxed" :class="{ 'textarea-error': workflowJsonError }" :placeholder="t('config.imageGeneration.workflowPlaceholder')" />
            <span v-if="workflowJsonError" class="mt-1 text-xs text-error">{{ workflowJsonError }}</span>
          </label>
          <div class="grid gap-3 md:grid-cols-2">
            <div v-for="field in comfyMappingFields" :key="field.key" class="rounded-box border border-base-300 bg-base-200/30 p-3">
              <div class="text-sm font-medium">{{ t(field.labelKey) }}</div>
              <label class="mt-2 grid gap-1">
                <span class="text-xs">{{ t("config.imageGeneration.nodeIds") }}</span>
                <input class="input input-bordered input-sm font-mono" :value="selectedProvider.comfyuiMapping[field.key].nodeIds.join(', ')" @input="setMappingNodeIds(field.key, ($event.target as HTMLInputElement).value)" />
              </label>
              <label class="mt-2 grid gap-1">
                <span class="text-xs">{{ t("config.imageGeneration.inputKey") }}</span>
                <input v-model="selectedProvider.comfyuiMapping[field.key].inputKey" class="input input-bordered input-sm font-mono" />
              </label>
            </div>
          </div>
          <label class="grid gap-1">
            <span class="text-sm font-medium">{{ t("config.imageGeneration.outputNodeIds") }}</span>
            <input class="input input-bordered input-sm font-mono text-xs" :value="selectedProvider.comfyuiMapping.outputNodeIds.join(', ')" @input="setOutputNodeIds(($event.target as HTMLInputElement).value)" />
           <span class="text-xs text-base-content/50">{{ t("config.imageGeneration.outputNodeIdsHint") }}</span>
          </label>
        </div>
        </div>
      </section>

      <section>
        <h3 class="mb-1 text-base font-semibold">{{ t("config.imageGeneration.testTitle") }}</h3>
        <p class="mb-3 text-xs text-base-content/60">{{ t("config.imageGeneration.testHint") }}</p>
        <div class="card border border-base-300 bg-base-100">
        <div class="card-body gap-4 p-4">
          <div v-if="imageDirty" class="alert alert-warning py-2 text-xs">
            <span>{{ t("config.imageGeneration.testSaveFirst") }}</span>
          </div>

          <!-- 上：参数一排（与 image_generate 工具的可选参数对齐） -->
          <div class="flex flex-wrap gap-3">
            <div class="grid content-start gap-1">
              <span class="text-xs font-medium text-base-content/60">{{ t("config.imageGeneration.testSize") }}</span>
              <select v-model="testResolution" class="select select-bordered select-sm w-40 font-mono">
                <option value="">{{ t("config.imageGeneration.testOptionModelDefault") }}</option>
                <option v-for="preset in testResolutionPresets" :key="preset" :value="preset">{{ preset }}</option>
              </select>
            </div>
          </div>

          <!-- 中：提示词，标题独立一行 + 全宽多行文本框 -->
          <div class="grid gap-1">
            <span class="text-sm font-medium">{{ t("config.imageGeneration.testPrompt") }}</span>
            <textarea v-model="testPrompt" class="textarea textarea-bordered min-h-24 w-full" :placeholder="t('config.imageGeneration.testPromptPlaceholder')" />
          </div>

          <!-- 下：生成按钮 + 常驻预览 -->
          <button class="btn btn-primary w-full" type="button" :disabled="!canRunImageTest" @click="runImageTest">
            <span v-if="testingImage" class="loading loading-spinner loading-sm" />
            <ImageIcon v-else class="h-4 w-4" />
            {{ testingImage ? t("config.imageGeneration.generating") : t("config.imageGeneration.generateTest") }}
          </button>

          <div v-if="testError" class="rounded-box border border-error/30 bg-error/5 px-3 py-2 text-xs text-error">{{ testError }}</div>

          <div class="overflow-hidden rounded-box border border-base-300 bg-base-200/40">
            <div v-if="testingImage" class="skeleton h-72 w-full rounded-none" />
            <img v-else-if="testPreviewDataUrl" :src="testPreviewDataUrl" :alt="firstTestImage?.revisedPrompt || testPrompt" class="mx-auto block max-h-96 object-contain" />
            <div v-else class="flex h-48 flex-col items-center justify-center gap-2 text-xs text-base-content/45">
              <ImageIcon class="h-8 w-8 opacity-40" />
              <span>{{ t("config.imageGeneration.previewEmpty") }}</span>
            </div>
          </div>

          <template v-if="!testingImage && testResult && firstTestImage">
            <div class="flex flex-wrap items-center gap-2 text-xs">
              <span class="badge badge-ghost badge-sm">{{ testResult.providerName }} · {{ testResult.model }}</span>
              <span class="badge badge-ghost badge-sm font-mono">{{ firstTestImage.width }}×{{ firstTestImage.height }}</span>
              <code class="min-w-0 flex-1 truncate rounded bg-base-200 px-2 py-1 text-[11px]" :title="firstTestImage.relativePath">{{ firstTestImage.relativePath }}</code>
              <button v-if="localFileSystemAvailable" class="btn btn-xs btn-ghost shrink-0" type="button" :disabled="copyingImage" @click="copyGeneratedImage(firstTestImage.relativePath)">
                <span v-if="copyingImage" class="loading loading-spinner loading-xs" />
                <ImageIcon v-else class="h-3.5 w-3.5" />
                {{ t("config.imageGeneration.copyImage") }}
              </button>
              <button class="btn btn-xs btn-ghost shrink-0" type="button" @click="copyGeneratedMarkdown(firstTestImage.markdown)">
                <Copy class="h-3.5 w-3.5" />
                {{ t("config.imageGeneration.copyMarkdown") }}
              </button>
            </div>
            <div v-if="firstTestImage.revisedPrompt" class="text-xs text-base-content/60">
              <span class="font-medium">{{ t("config.imageGeneration.revisedPrompt") }}：</span>{{ firstTestImage.revisedPrompt }}
            </div>
          </template>
        </div>
        </div>
      </section>
    </div>

    <div v-else class="card border border-dashed border-base-300 bg-base-100">
      <div class="card-body items-center justify-center text-center text-sm text-base-content/55">
        <ImageIcon class="h-10 w-10 opacity-35" />
        <p>{{ t("config.imageGeneration.selectOrAddProvider") }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { ArrowUpToLine, ChevronDown, Copy, Eye, EyeOff, Image as ImageIcon, Plus, RotateCcw, Save, Trash2, Workflow } from "@lucide/vue";
import type {
  AppConfig,
  ComfyUiWorkflowMapping,
  ImageGenerationProviderKind,
  ImageGenerationResult,
} from "../../../../types/app";
import {
  copyTransportChatImageToClipboard,
  getTransportCapabilities,
  invokeTauri,
  readTransportChatImage,
} from "../../../../services/tauri-api";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import type { ConfigTemplateGroup } from "../../components/config-template";
import {
  createImageGenerationModel,
  createImageGenerationProvider,
  imageGenerationEndpointId,
  normalizeImageGenerationModelId,
  normalizeImageGenerationProviders,
} from "../../utils/image-generation-config";

type ComfyInputMappingKey = Exclude<keyof ComfyUiWorkflowMapping, "outputNodeIds">;

const props = defineProps<{
  config: AppConfig;
  configDirty: boolean;
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  lastSavedConfigJson: string;
  setStatusAction: (text: string) => void;
}>();

const { t } = useI18n();
const selectedProviderId = ref("");
const testPrompt = ref("");
const localFileSystemAvailable = getTransportCapabilities().localFileSystem;
// 与 AI 工具 image_generate 的可选参数对齐：仅 resolution，留空表示用模型默认值
const testResolution = ref("");
const testResolutionPresets = ["512x512", "1024x1024", "1536x1024", "1024x1536", "2K", "4K"];
const testingImage = ref(false);
const copyingImage = ref(false);
const testError = ref("");
const testResult = ref<ImageGenerationResult | null>(null);
const testPreviewDataUrl = ref("");
const firstTestImage = computed(() => testResult.value?.images[0] || null);
const activeImageModelPickerId = ref("");
const imageModelSearch = ref("");
const showImageApiKeys = ref<Record<string, Record<number, boolean>>>({});
let localSeed = 0;

const providerTypeOptions: Array<{ value: ImageGenerationProviderKind; label: string }> = [
  { value: "comfyui", label: "ComfyUI" },
  { value: "codex", label: "OpenAI Codex" },
  { value: "openai", label: "OpenAI" },
  { value: "xai", label: "xAI Grok Imagine" },
  { value: "seedream", label: "Seedance / Seedream" },
  { value: "gemini", label: "Gemini Nano Banana 2" },
];

const imageModelOptions = computed(() => {
  const provider = selectedProvider.value;
  if (!provider) return [];
  const defaults: Record<ImageGenerationProviderKind, string[]> = {
    comfyui: [],
    codex: ["gpt-image-2"],
    openai: ["gpt-image-2"],
    xai: ["grok-imagine-image-quality", "grok-imagine-image"],
    seedream: ["doubao-seedream-5-0-pro-260628"],
    gemini: ["gemini-3.1-flash-image"],
  };
  return Array.from(new Set([...defaults[provider.providerType], ...provider.models.map((model) => model.model || model.id)].filter(Boolean)));
});

const filteredImageModelOptions = computed(() => {
  const query = imageModelSearch.value.trim().toLowerCase();
  return imageModelOptions.value.filter((option) => !query || option.toLowerCase().includes(query));
});

const comfyMappingFields: Array<{ key: ComfyInputMappingKey; labelKey: string }> = [
  { key: "prompt", labelKey: "config.imageGeneration.mappingPrompt" },
  { key: "negativePrompt", labelKey: "config.imageGeneration.mappingNegativePrompt" },
  { key: "model", labelKey: "config.imageGeneration.mappingModel" },
  { key: "width", labelKey: "config.imageGeneration.mappingWidth" },
  { key: "height", labelKey: "config.imageGeneration.mappingHeight" },
  { key: "seed", labelKey: "config.imageGeneration.mappingSeed" },
  { key: "steps", labelKey: "config.imageGeneration.mappingSteps" },
  { key: "inputImage", labelKey: "config.imageGeneration.mappingInputImage" },
  { key: "maskImage", labelKey: "config.imageGeneration.mappingMaskImage" },
];

const providers = computed(() => props.config.imageProviders || []);
const selectedProvider = computed(() => (
  providers.value.find((provider) => provider.id === selectedProviderId.value) || null
));
const codexApiProviders = computed(() => (
  (props.config.apiProviders || []).filter((provider) => provider.requestFormat === "codex" && !provider.deprecated)
));

const providerTemplateValues = computed<Record<string, unknown>>({
  get: () => {
    const provider = selectedProvider.value;
    if (!provider) return {};
    return {
      providerName: provider.name,
      providerType: provider.providerType,
      baseUrl: provider.baseUrl,
      codexApiProviderId: provider.codexApiProviderId || "",
      timeoutSeconds: provider.timeoutSeconds,
      watermark: provider.watermark,
    };
  },
  set: (values) => {
    const provider = selectedProvider.value;
    if (!provider) return;
    if (typeof values.providerType === "string" && providerTypeOptions.some((item) => item.value === values.providerType)) {
      provider.providerType = values.providerType as ImageGenerationProviderKind;
      if (provider.providerType === "codex" && !provider.codexApiProviderId) provider.codexApiProviderId = codexApiProviders.value[0]?.id;
    }
    if (typeof values.providerName === "string") provider.name = values.providerName;
    if (typeof values.baseUrl === "string") provider.baseUrl = values.baseUrl;
    if (typeof values.codexApiProviderId === "string") provider.codexApiProviderId = values.codexApiProviderId;
    if (typeof values.timeoutSeconds === "number" && Number.isFinite(values.timeoutSeconds)) provider.timeoutSeconds = values.timeoutSeconds;
    if (typeof values.watermark === "boolean") provider.watermark = values.watermark;
  },
});

const providerTemplateGroups = computed<ConfigTemplateGroup[]>(() => {
  const provider = selectedProvider.value;
  if (!provider) return [];
  const endpointField = provider.providerType === "codex"
    ? { key: "codexApiProviderId", label: t("config.imageGeneration.codexApiProvider"), description: t("config.imageGeneration.codexApiProviderHint"), type: "select" as const, options: [{ value: "", label: t("config.imageGeneration.codexApiProviderMissing") }, ...codexApiProviders.value.map((item) => ({ value: item.id, label: item.name }))] }
    : { key: "baseUrl", label: t("config.imageGeneration.baseUrl"), type: "text" as const };
  const fields = [
    { key: "providerType", label: t("config.imageGeneration.providerType"), type: "select" as const, options: providerTypeOptions.map((item) => ({ value: item.value, label: item.label })) },
    { key: "providerName", label: t("config.imageGeneration.providerName"), type: "text" as const },
    endpointField,
    { key: "timeoutSeconds", label: t("config.imageGeneration.timeoutSeconds"), type: "number" as const, min: 10, max: 600 },
  ];
  if (provider.providerType === "seedream") fields.push({ key: "watermark", label: t("config.imageGeneration.watermark"), description: t("config.imageGeneration.watermarkHint"), type: "toggle" as const } as never);
  const rows: ConfigTemplateGroup["rows"] = fields.map((field) => ({ items: [field] }));
  return [{ title: t("config.imageGeneration.providerSettings"), rows }];
});

function imageConfigSnapshot(config: Partial<AppConfig>) {
  const imageProviders = normalizeImageGenerationProviders(config.imageProviders);
  return {
    imageGenerationModelId: normalizeImageGenerationModelId(config.imageGenerationModelId, imageProviders),
    imageProviders,
  };
}

const savedImageSnapshot = computed(() => {
  try {
    return imageConfigSnapshot(JSON.parse(String(props.lastSavedConfigJson || "{}")) as Partial<AppConfig>);
  } catch {
    return imageConfigSnapshot({ imageProviders: [] });
  }
});
const currentImageSnapshot = computed(() => imageConfigSnapshot(props.config));
const imageDirty = computed(() => (
  JSON.stringify(currentImageSnapshot.value) !== JSON.stringify(savedImageSnapshot.value)
));
const canRunImageTest = computed(() => (
  !imageDirty.value
  && !testingImage.value
  && testPrompt.value.trim().length > 0
  && !!props.config.imageGenerationModelId
));

const workflowJsonError = computed(() => {
  const provider = selectedProvider.value;
  if (!provider || provider.providerType !== "comfyui") return "";
  const raw = provider.comfyuiWorkflowJson.trim();
  if (!raw) return t("config.imageGeneration.workflowRequired");
  try {
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return t("config.imageGeneration.workflowObjectRequired");
    }
    return "";
  } catch (error) {
    return t("config.imageGeneration.workflowInvalid", { error: error instanceof Error ? error.message : String(error) });
  }
});

const enabledWorkflowError = computed(() => {
  for (const provider of providers.value) {
    if (!provider.enabled || provider.deprecated || provider.providerType !== "comfyui") continue;
    if (!provider.comfyuiMapping.prompt.nodeIds.length || !provider.comfyuiMapping.prompt.inputKey.trim()) {
      return t("config.imageGeneration.promptMappingRequiredFor", { name: provider.name });
    }
    const raw = provider.comfyuiWorkflowJson.trim();
    if (!raw) return t("config.imageGeneration.workflowRequiredFor", { name: provider.name });
    try {
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        return t("config.imageGeneration.workflowRequiredFor", { name: provider.name });
      }
    } catch {
      return t("config.imageGeneration.workflowRequiredFor", { name: provider.name });
    }
  }
  for (const provider of providers.value) {
    if (!provider.enabled || provider.deprecated || provider.providerType !== "codex") continue;
    const linked = String(provider.codexApiProviderId || "").trim();
    if (!linked || !codexApiProviders.value.some((item) => item.id === linked)) {
      return t("config.imageGeneration.codexProviderRequired", { name: provider.name });
    }
  }
  return "";
});

watch(
  providers,
  (value) => {
    if (value.some((provider) => provider.id === selectedProviderId.value)) return;
    selectedProviderId.value = value[0]?.id || "";
  },
  { immediate: true },
);

function nextSeed(): string {
  localSeed += 1;
  return `${Date.now()}-${localSeed}`;
}

function providerTypeLabel(kind: ImageGenerationProviderKind): string {
  return providerTypeOptions.find((option) => option.value === kind)?.label || kind;
}

function clearInvalidDefaultModel() {
  props.config.imageGenerationModelId = normalizeImageGenerationModelId(
    props.config.imageGenerationModelId,
    providers.value,
  );
}

function addProvider() {
  const provider = createImageGenerationProvider("openai", nextSeed());
  if (provider.providerType === "codex") {
    provider.codexApiProviderId = codexApiProviders.value[0]?.id;
  }
  props.config.imageProviders.push(provider);
  selectedProviderId.value = provider.id;
  if (!props.config.imageGenerationModelId && provider.models[0]) {
    props.config.imageGenerationModelId = imageGenerationEndpointId(provider.id, provider.models[0].id);
  }
}

function setCodexApiProvider(value: string) {
  if (!selectedProvider.value || selectedProvider.value.providerType !== "codex") return;
  selectedProvider.value.codexApiProviderId = value.trim() || undefined;
}

function removeSelectedProvider() {
  const provider = selectedProvider.value;
  if (!provider) return;
  if (!window.confirm(t("config.imageGeneration.confirmRemoveProvider", { name: provider.name }))) return;
  const index = props.config.imageProviders.findIndex((item) => item.id === provider.id);
  if (index >= 0) props.config.imageProviders.splice(index, 1);
  clearInvalidDefaultModel();
}

function addApiKey() {
  const provider = selectedProvider.value;
  if (!provider) return;
  provider.apiKeys.push("");
}

function removeApiKey(index: number) {
  const provider = selectedProvider.value;
  if (!provider || provider.apiKeys.length <= 1) return;
  provider.apiKeys.splice(index, 1);
}

function pinApiKeyToTop(index: number) {
  const provider = selectedProvider.value;
  if (!provider || index <= 0 || index >= provider.apiKeys.length) return;
  const [key] = provider.apiKeys.splice(index, 1);
  provider.apiKeys.unshift(key);
}

function toggleImageApiKeyVisible(providerId: string, index: number) {
  showImageApiKeys.value = {
    ...showImageApiKeys.value,
    [providerId]: {
      ...(showImageApiKeys.value[providerId] || {}),
      [index]: !showImageApiKeys.value[providerId]?.[index],
    },
  };
}

function addModel() {
  const provider = selectedProvider.value;
  if (!provider) return;
  const model = createImageGenerationModel(provider.providerType, nextSeed());
  provider.models.push(model);
  if (!props.config.imageGenerationModelId && provider.enabled) {
    props.config.imageGenerationModelId = imageGenerationEndpointId(provider.id, model.id);
  }
}

function removeModel(modelId: string) {
  const provider = selectedProvider.value;
  if (!provider) return;
  const model = provider.models.find((item) => item.id === modelId);
  if (!model) return;
  if (!window.confirm(t("config.imageGeneration.confirmRemoveModel", { name: model.name || model.model || model.id }))) return;
  const index = provider.models.findIndex((item) => item.id === modelId);
  if (index >= 0) provider.models.splice(index, 1);
  clearInvalidDefaultModel();
}

function updateModelIdentifier(previousId: string, event: Event) {
  const provider = selectedProvider.value;
  const model = provider?.models.find((item) => item.id === previousId);
  if (!provider || !model) return;
  const input = event.target as HTMLInputElement;
  const value = input.value;
  const nextId = String(value || "").trim();
  if (!nextId || nextId.includes("::") || provider.models.some((item) => item !== model && item.id.toLowerCase() === nextId.toLowerCase())) {
    input.value = previousId;
    props.setStatusAction(t("config.imageGeneration.invalidModelId"));
    return;
  }
  const previousEndpointId = imageGenerationEndpointId(provider.id, previousId);
  model.id = nextId;
  model.model = nextId;
  if (props.config.imageGenerationModelId === previousEndpointId) {
    props.config.imageGenerationModelId = imageGenerationEndpointId(provider.id, nextId);
  }
}

function toggleImageModelPicker(modelId: string) {
  activeImageModelPickerId.value = activeImageModelPickerId.value === modelId ? "" : modelId;
  imageModelSearch.value = "";
}

function selectImageModel(previousId: string, value: string) {
  const provider = selectedProvider.value;
  const model = provider?.models.find((item) => item.id === previousId);
  if (!provider || !model || !value) return;
  model.model = value;
  model.id = value;
  activeImageModelPickerId.value = "";
  imageModelSearch.value = "";
}

function splitNodeIds(value: string): string[] {
  return value.split(/[，,\s]+/).map((item) => item.trim()).filter(Boolean);
}

function setMappingNodeIds(key: ComfyInputMappingKey, value: string) {
  if (!selectedProvider.value) return;
  selectedProvider.value.comfyuiMapping[key].nodeIds = splitNodeIds(value);
}

function setOutputNodeIds(value: string) {
  if (!selectedProvider.value) return;
  selectedProvider.value.comfyuiMapping.outputNodeIds = splitNodeIds(value);
}

function restoreImageConfig() {
  props.config.imageProviders = normalizeImageGenerationProviders(savedImageSnapshot.value.imageProviders);
  props.config.imageGenerationModelId = savedImageSnapshot.value.imageGenerationModelId;
  props.setStatusAction(t("config.imageGeneration.restored"));
}

async function saveImageConfig() {
  if (!imageDirty.value) return;
  if (enabledWorkflowError.value) {
    props.setStatusAction(enabledWorkflowError.value);
    return;
  }
  const saved = await Promise.resolve(props.saveConfigAction());
  if (!saved) return;
  props.setStatusAction(t("config.imageGeneration.saved"));
}

async function runImageTest() {
  if (!canRunImageTest.value) return;
  testingImage.value = true;
  testError.value = "";
  testResult.value = null;
  testPreviewDataUrl.value = "";
  const request: Record<string, unknown> = {
    prompt: testPrompt.value.trim(),
    n: 1,
  };
  const resolution = testResolution.value.trim();
  if (resolution) request.size = resolution;
  try {
    const result = await invokeTauri<ImageGenerationResult>("generate_image", { request });
    testResult.value = result;
    const firstImage = result.images?.[0];
    if (firstImage?.relativePath) {
      // 后端预览命令只认 {Assistant Space} 前缀，裸相对路径会按进程工作目录解析导致找不到文件
      const preview = await readTransportChatImage({
        path: `{Assistant Space}/${firstImage.relativePath}`,
        maxEdge: 768,
      });
      testPreviewDataUrl.value = String(preview?.dataUrl || "");
    }
    props.setStatusAction(t("config.imageGeneration.testCompleted"));
  } catch (error) {
    testError.value = String(error instanceof Error ? error.message : error || t("config.imageGeneration.testFailed"));
    props.setStatusAction(t("config.imageGeneration.testFailed"));
  } finally {
    testingImage.value = false;
  }
}

async function copyGeneratedMarkdown(markdown: string) {
  const value = String(markdown || "").trim();
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    props.setStatusAction(t("config.imageGeneration.markdownCopied"));
  } catch (error) {
    testError.value = String(error instanceof Error ? error.message : error || t("config.imageGeneration.copyFailed"));
  }
}

async function copyGeneratedImage(relativePath: string) {
  const value = String(relativePath || "").trim();
  if (!value || copyingImage.value) return;
  copyingImage.value = true;
  try {
    await copyTransportChatImageToClipboard(`{Assistant Space}/${value}`);
    props.setStatusAction(t("config.imageGeneration.imageCopied"));
  } catch (error) {
    testError.value = String(error instanceof Error ? error.message : error || t("config.imageGeneration.copyFailed"));
  } finally {
    copyingImage.value = false;
  }
}
</script>
