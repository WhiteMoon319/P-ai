package com.whitemoon319.pai.model

import com.google.gson.JsonElement
import com.google.gson.annotations.SerializedName

// ---------------- 设置（模型与供应商 / 聊天设置 / 工具 / 关于） ----------------

/** load_config 返回的 AppConfig（只解析 Android 设置页所需子集）。 */
data class AppConfig(
    @SerializedName("apiConfigs") val apiConfigs: List<ApiConfig> = emptyList(),
    @SerializedName("assistantDepartmentApiConfigId") val assistantDepartmentApiConfigId: String? = null,
    @SerializedName("selectedApiConfigId") val selectedApiConfigId: String? = null,
    @SerializedName("webAccessEnabled") val webAccessEnabled: Boolean? = null,
    @SerializedName("webAccessPort") val webAccessPort: Int? = null,
    @SerializedName("webAccessPassword") val webAccessPassword: String? = null,
    @SerializedName("sttApiConfigId") val sttApiConfigId: String? = null,
    @SerializedName("sttAutoSend") val sttAutoSend: Boolean? = null,
    @SerializedName("messageNotificationEnabled") val messageNotificationEnabled: Boolean? = null,
    @SerializedName("messageNotificationSoundEnabled") val messageNotificationSoundEnabled: Boolean? = null,
    @SerializedName("desktopOperationNoticeEnabled") val desktopOperationNoticeEnabled: Boolean? = null,
    @SerializedName("uiLanguage") val uiLanguage: String? = null,
    @SerializedName("uiSizeScale") val uiSizeScale: Int? = null,
    @SerializedName("departments") val departments: List<DepartmentConfig> = emptyList(),
)

/** 部门配置（DepartmentConfig），只读展示用。 */
data class DepartmentConfig(
    val id: String? = null,
    val name: String? = null,
    val summary: String? = null,
    val guide: String? = null,
    @SerializedName("apiConfigIds") val apiConfigIds: List<String> = emptyList(),
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
    @SerializedName("modelFailureFallbackEnabled") val modelFailureFallbackEnabled: Boolean = false,
    @SerializedName("agentIds") val agentIds: List<String> = emptyList(),
    @SerializedName("childDepartmentIds") val childDepartmentIds: List<String> = emptyList(),
    @SerializedName("isBuiltInAssistant") val isBuiltInAssistant: Boolean = false,
    val source: String? = null,
    val scope: String? = null,
)

/** 会话级 API 设置（含 STT/视觉/工具审查供应商），save_conversation_api_settings 用。 */
data class ConversationApiSettings(
    @SerializedName("assistantDepartmentApiConfigId", alternate = ["chatApiConfigId"])
    val assistantDepartmentApiConfigId: String? = null,
    @SerializedName("visionApiConfigId") val visionApiConfigId: String? = null,
    @SerializedName("toolReviewApiConfigId") val toolReviewApiConfigId: String? = null,
    @SerializedName("sttApiConfigId") val sttApiConfigId: String? = null,
    @SerializedName("sttAutoSend") val sttAutoSend: Boolean = false,
)

/** 单个 API 配置（供应商）。 */
data class ApiConfig(
    val id: String? = null,
    val name: String? = null,
    @SerializedName("requestFormat") val requestFormat: String? = null,
    @SerializedName("allowConcurrentRequests") val allowConcurrentRequests: Boolean = true,
    @SerializedName("maxConcurrentRequests") val maxConcurrentRequests: Int? = null,
    @SerializedName("enableText") val enableText: Boolean = true,
    @SerializedName("enableImage") val enableImage: Boolean = false,
    @SerializedName("enableAudio") val enableAudio: Boolean = false,
    @SerializedName("enableVideo") val enableVideo: Boolean = false,
    @SerializedName("enableTools") val enableTools: Boolean = true,
    @SerializedName("baseUrl") val baseUrl: String? = null,
    @SerializedName("apiKey") val apiKey: String? = null,
    val model: String? = null,
    @SerializedName("reasoningEffort") val reasoningEffort: String? = null,
    val temperature: Double = 0.7,
    @SerializedName("customTemperatureEnabled") val customTemperatureEnabled: Boolean = false,
    @SerializedName("contextWindowTokens") val contextWindowTokens: Int = 128000,
    @SerializedName("maxOutputTokens") val maxOutputTokens: Int = 8192,
    @SerializedName("customMaxOutputTokensEnabled") val customMaxOutputTokensEnabled: Boolean = false,
    @SerializedName("failureRetryCount") val failureRetryCount: Int = 0,
)

/** load_chat_settings / save_chat_settings 的 ChatSettings。 */
data class ChatSettings(
    @SerializedName("assistantDepartmentAgentId") val assistantDepartmentAgentId: String? = null,
    @SerializedName("userAlias") val userAlias: String? = null,
    @SerializedName("responseStyleId") val responseStyleId: String? = null,
    @SerializedName("pdfReadMode") val pdfReadMode: String? = null,
    @SerializedName("instructionPresets") val instructionPresets: List<PromptCommandPreset> = emptyList(),
)

/** 人设/代理（AgentProfile），load_agents / save_agents 用。 */
data class AgentProfile(
    val id: String? = null,
    val name: String? = null,
    @SerializedName("systemPrompt") val systemPrompt: String? = null,
    @SerializedName("createdAt") val createdAt: String? = null,
    @SerializedName("updatedAt") val updatedAt: String? = null,
    @SerializedName("avatarPath") val avatarPath: String? = null,
    @SerializedName("isBuiltInUser") val isBuiltInUser: Boolean = false,
    @SerializedName("isBuiltInSystem") val isBuiltInSystem: Boolean = false,
    @SerializedName("privateMemoryEnabled") val privateMemoryEnabled: Boolean = false,
    @SerializedName("memoryRecallMode") val memoryRecallMode: String? = null,
    val source: String? = null,
    val scope: String? = null,
)

/** save_agents 输入。 */
data class SaveAgentsInput(
    val agents: List<AgentProfile> = emptyList(),
)

data class PromptCommandPreset(
    val id: String? = null,
    val name: String? = null,
    val prompt: String? = null,
)

/** check_tools_status 的请求 input。 */
data class CheckToolsStatusInput(
    @SerializedName("agentId") val agentId: String? = null,
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
)

/** check_tools_status 返回的单条工具状态。 */
data class ToolLoadStatus(
    val id: String? = null,
    val status: String? = null,
    val detail: String? = null,
)

/** set_department_primary_api_config 的请求 input。 */
data class SetDepartmentPrimaryApiConfigInput(
    @SerializedName("departmentId") val departmentId: String? = null,
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
)

/** app.bootstrapSnapshot 返回（只取设置页需要的部分）。 */
data class BootstrapSnapshot(
    val config: AppConfig? = null,
    val chatSettings: ChatSettings? = null,
)

/** Android 沙盒工作区状态。 */
data class AndroidWorkspaceStatus(
    val state: String? = null,
    @SerializedName("rootPath") val rootPath: String? = null,
    @SerializedName("llmWorkspaceRoot") val llmWorkspaceRoot: String? = null,
    @SerializedName("runtimeRoot") val runtimeRoot: String? = null,
    @SerializedName("initializedAt") val initializedAt: String? = null,
    @SerializedName("updatedAt") val updatedAt: String? = null,
    @SerializedName("lastError") val lastError: String? = null,
    val version: Int = 0,
    @SerializedName("runtimeVersion") val runtimeVersion: String? = null,
    @SerializedName("downloadBytes") val downloadBytes: Long? = null,
    @SerializedName("downloadTotalBytes") val downloadTotalBytes: Long? = null,
    @SerializedName("downloadStage") val downloadStage: String? = null,
) {
    // 后端枚举 AndroidWorkspaceStateKind 序列化为 snake_case（ready/downloading/not_downloaded）
    val isReady: Boolean get() = state == "ready" || state == "Ready"
    val isDownloading: Boolean get() = state == "downloading" || state == "Downloading"
    val isNotDownloaded: Boolean get() = state == "not_downloaded" || state == "NotDownloaded"
}

/** api_config.delete 的请求 input。 */
data class ApiConfigDeleteInput(
    val id: String? = null,
)
