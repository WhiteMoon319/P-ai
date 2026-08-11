use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::domain::constants::{
    APP_DATA_SCHEMA_VERSION, ASSISTANT_DEPARTMENT_ID, DEFAULT_AGENT_ID, SYSTEM_NOTIFICATION_CONVERSATION_ID,
    USER_PERSONA_ID,
};
use crate::core::domain::runtime_defaults::{
    default_agent, default_deputy_agent, default_system_persona, default_user_persona,
};
use crate::core::domain::types_chat::{AgentProfile, Conversation, ConversationArchive};
use crate::core::domain::types_config::{
    default_assistant_department, default_deputy_department, normalize_department_child_ids,
    AppConfig, DepartmentConfig, DepartmentPermissionControl, RemoteImPlatform,
    ShellWorkspaceConfig,
};
use crate::core::domain::types_foundation::{
    default_background_voice_screenshot_keywords, default_background_voice_screenshot_mode,
    default_pdf_read_mode, default_response_style_id,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageTextCacheEntry {
    pub hash: String,
    #[serde(alias = "visionApiId")]
    pub model_api_id: String,
    #[serde(default = "default_media_cache_entry_type")]
    pub media_type: String,
    #[serde(default)]
    pub description: String,
    pub text: String,
    pub updated_at: String,
}

pub fn default_media_cache_entry_type() -> String {
    "image".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfTextCacheEntry {
    pub file_hash: String,
    pub file_path: String,
    pub file_name: String,
    pub extracted_text: String,
    pub total_pages: u32,
    pub extracted_pages: u32,
    pub is_truncated: bool,
    pub conversation_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfImageCacheEntry {
    pub file_hash: String,
    pub file_path: String,
    pub file_name: String,
    pub total_pages: u32,
    pub rendered_pages: u32,
    pub dpi: u32,
    pub images: Vec<PdfRenderedImage>,
    pub conversation_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfRenderedImage {
    pub page_index: usize,
    pub width: u32,
    pub height: u32,
    pub bytes_base64: String,
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub id: String,
    #[serde(default)]
    pub memory_no: Option<u64>,
    #[serde(default, alias = "memoryType")]
    pub memory_type: String,
    #[serde(default, alias = "content")]
    pub judgment: String,
    #[serde(default)]
    pub reasoning: String,
    #[serde(default, alias = "keywords")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub owner_agent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MemoryEntry {
    pub fn display_id(&self) -> String {
        self.memory_no
            .map(|value| value.to_string())
            .unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptCommandPreset {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSectionOrders {
    #[serde(default)]
    pub local: Vec<String>,
    #[serde(default)]
    pub contact: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    #[serde(default)]
    pub data_migration_version: u32,
    #[serde(default, alias = "messageStoreMigrationVersion")]
    pub message_store_migration_version: u32,
    pub agents: Vec<AgentProfile>,
    #[serde(
        default = "default_assistant_department_agent_id",
        alias = "selectedAgentId",
        alias = "selected_agent_id"
    )]
    pub assistant_department_agent_id: String,
    #[serde(default = "default_user_alias")]
    pub user_alias: String,
    #[serde(default = "default_response_style_id")]
    pub response_style_id: String,
    #[serde(default = "default_pdf_read_mode")]
    pub pdf_read_mode: String,
    #[serde(default = "default_background_voice_screenshot_keywords")]
    pub background_voice_screenshot_keywords: String,
    #[serde(default = "default_background_voice_screenshot_mode")]
    pub background_voice_screenshot_mode: String,
    #[serde(default)]
    pub instruction_presets: Vec<PromptCommandPreset>,
    #[serde(
        default,
        rename = "systemNotificationConversationId",
        alias = "mainConversationId",
        alias = "main_conversation_id"
    )]
    pub main_conversation_id: Option<String>,
    #[serde(default)]
    pub pinned_conversation_ids: Vec<String>,
    #[serde(default)]
    pub conversation_section_orders: ConversationSectionOrders,
    pub conversations: Vec<Conversation>,
    #[serde(default)]
    pub image_text_cache: Vec<ImageTextCacheEntry>,
    #[serde(default)]
    pub pdf_text_cache: Vec<PdfTextCacheEntry>,
    #[serde(default)]
    pub pdf_image_cache: Vec<PdfImageCacheEntry>,
    #[serde(default)]
    pub remote_im_contacts: Vec<RemoteImContact>,
    #[serde(default)]
    pub remote_im_contact_checkpoints: Vec<RemoteImContactCheckpoint>,
    #[serde(default)]
    pub archived_conversations: Vec<ConversationArchive>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: APP_DATA_SCHEMA_VERSION,
            data_migration_version: 0,
            message_store_migration_version: 0,
            agents: vec![
                default_agent(),
                default_deputy_agent(),
                default_user_persona(),
                default_system_persona(),
            ],
            assistant_department_agent_id: default_assistant_department_agent_id(),
            user_alias: default_user_alias(),
            response_style_id: default_response_style_id(),
            pdf_read_mode: default_pdf_read_mode(),
            background_voice_screenshot_keywords: default_background_voice_screenshot_keywords(),
            background_voice_screenshot_mode: default_background_voice_screenshot_mode(),
            instruction_presets: Vec::new(),
            main_conversation_id: None,
            pinned_conversation_ids: Vec::new(),
            conversation_section_orders: ConversationSectionOrders::default(),
            conversations: Vec::new(),
            image_text_cache: Vec::new(),
            pdf_text_cache: Vec::new(),
            pdf_image_cache: Vec::new(),
            remote_im_contacts: Vec::new(),
            remote_im_contact_checkpoints: Vec::new(),
            archived_conversations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImGroupMemberInfo {
    pub user_id: String,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub card: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImContact {
    pub id: String,
    pub channel_id: String,
    pub platform: RemoteImPlatform,
    pub remote_contact_type: String,
    pub remote_contact_id: String,
    #[serde(default)]
    pub remote_contact_name: String,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub remark_name: String,
    #[serde(default)]
    pub allow_send: bool,
    #[serde(default)]
    pub allow_send_files: bool,
    #[serde(default)]
    pub allow_receive: bool,
    #[serde(default = "default_remote_im_contact_activation_mode")]
    pub activation_mode: String,
    #[serde(default)]
    pub activation_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_mute_keywords")]
    pub mute_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_unmute_keywords")]
    pub unmute_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_patience_seconds")]
    pub patience_seconds: u64,
    #[serde(default = "default_remote_im_contact_mute_duration_seconds")]
    pub mute_duration_seconds: u64,
    #[serde(default)]
    pub activation_cooldown_seconds: u64,
    #[serde(default = "default_remote_im_contact_route_mode")]
    pub route_mode: String,
    #[serde(default)]
    pub bound_department_id: Option<String>,
    #[serde(default)]
    pub bound_agent_id: Option<String>,
    #[serde(default)]
    pub bound_conversation_id: Option<String>,
    #[serde(default = "default_remote_im_contact_processing_mode")]
    pub processing_mode: String,
    #[serde(default = "default_remote_im_contact_response_strategy")]
    pub response_strategy: String,
    #[allow(dead_code)]
    #[serde(default = "default_remote_im_contact_response_guidance", skip_serializing)]
    pub response_guidance: String,
    #[serde(default = "default_remote_im_contact_blocked_message_prefixes")]
    pub blocked_message_prefixes: Vec<String>,
    #[serde(default)]
    pub group_reply_pacing: RemoteImGroupReplyPacing,
    #[serde(default)]
    pub last_activated_at: Option<String>,
    #[serde(default)]
    pub last_message_at: Option<String>,
    #[serde(default)]
    pub dingtalk_session_webhook: Option<String>,
    #[serde(default)]
    pub dingtalk_session_webhook_expired_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub onebot_group_members: Vec<RemoteImGroupMemberInfo>,
    #[serde(default)]
    pub shell_workspaces: Vec<ShellWorkspaceConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImGroupReplyPacing {
    #[serde(default = "default_remote_im_assistant_debounce_seconds")]
    pub assistant_debounce_seconds: u64,
    #[serde(default = "default_remote_im_secretary_inspection_seconds")]
    pub secretary_inspection_seconds: u64,
    #[serde(default = "default_remote_im_reply_cooldown_seconds")]
    pub reply_cooldown_seconds: u64,
    #[serde(default = "default_remote_im_inspection_jitter_ratio")]
    pub inspection_jitter_ratio: f64,
    #[serde(default = "default_remote_im_maximum_energy")]
    pub maximum_energy: f64,
    #[serde(default = "default_remote_im_base_reply_energy_cost")]
    pub base_reply_energy_cost: f64,
    #[serde(default = "default_remote_im_energy_cost_per_character")]
    pub energy_cost_per_character: f64,
    #[serde(default = "default_remote_im_energy_recovery_per_second")]
    pub energy_recovery_per_second: f64,
    #[serde(default = "default_remote_im_positive_energy_phrases")]
    pub positive_energy_phrases: Vec<String>,
    #[serde(default = "default_remote_im_negative_energy_phrases")]
    pub negative_energy_phrases: Vec<String>,
    #[serde(default = "default_remote_im_positive_energy_delta")]
    pub positive_energy_delta: f64,
    #[serde(default = "default_remote_im_negative_energy_delta")]
    pub negative_energy_delta: f64,
    #[serde(default = "default_remote_im_normal_reply_max_chars")]
    pub normal_reply_max_chars: u32,
    #[serde(default = "default_remote_im_focus_reply_max_chars")]
    pub focus_reply_max_chars: u32,
    #[serde(default = "default_remote_im_focus_instructions")]
    pub focus_instructions: Vec<String>,
}

impl Default for RemoteImGroupReplyPacing {
    fn default() -> Self {
        Self {
            assistant_debounce_seconds: default_remote_im_assistant_debounce_seconds(),
            secretary_inspection_seconds: default_remote_im_secretary_inspection_seconds(),
            reply_cooldown_seconds: default_remote_im_reply_cooldown_seconds(),
            inspection_jitter_ratio: default_remote_im_inspection_jitter_ratio(),
            maximum_energy: default_remote_im_maximum_energy(),
            base_reply_energy_cost: default_remote_im_base_reply_energy_cost(),
            energy_cost_per_character: default_remote_im_energy_cost_per_character(),
            energy_recovery_per_second: default_remote_im_energy_recovery_per_second(),
            positive_energy_phrases: default_remote_im_positive_energy_phrases(),
            negative_energy_phrases: default_remote_im_negative_energy_phrases(),
            positive_energy_delta: default_remote_im_positive_energy_delta(),
            negative_energy_delta: default_remote_im_negative_energy_delta(),
            normal_reply_max_chars: default_remote_im_normal_reply_max_chars(),
            focus_reply_max_chars: default_remote_im_focus_reply_max_chars(),
            focus_instructions: default_remote_im_focus_instructions(),
        }
    }
}

/// 渠道统一的静态行为参数。
///
/// 联系人仅保留路由、应答策略和运行时账本；这里的值是该渠道全部联系人的
/// 消息过滤、闭嘴、什么时候应该回答、在场和群聊巡检策略的唯一真值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImChannelBehaviorSettings {
    #[serde(default = "default_remote_im_contact_response_guidance")]
    pub response_guidance: String,
    #[serde(default = "default_remote_im_contact_blocked_message_prefixes")]
    pub blocked_message_prefixes: Vec<String>,
    #[serde(default = "default_remote_im_contact_mute_keywords")]
    pub mute_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_unmute_keywords")]
    pub unmute_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_patience_seconds")]
    pub patience_seconds: u64,
    #[serde(default = "default_remote_im_contact_mute_duration_seconds")]
    pub mute_duration_seconds: u64,
    #[serde(default)]
    pub activation_cooldown_seconds: u64,
    #[serde(default)]
    pub group_reply_pacing: RemoteImGroupReplyPacing,
}

impl Default for RemoteImChannelBehaviorSettings {
    fn default() -> Self {
        Self {
            response_guidance: default_remote_im_contact_response_guidance(),
            blocked_message_prefixes: default_remote_im_contact_blocked_message_prefixes(),
            mute_keywords: default_remote_im_contact_mute_keywords(),
            unmute_keywords: default_remote_im_contact_unmute_keywords(),
            patience_seconds: default_remote_im_contact_patience_seconds(),
            mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
            activation_cooldown_seconds: 0,
            group_reply_pacing: RemoteImGroupReplyPacing::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImGroupReplyDeliveryMarker {
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub boundary_message_id: String,
    #[serde(default)]
    pub outbound_key: String,
    #[serde(default)]
    pub final_text: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub platform_message_id: Option<String>,
    #[serde(default)]
    pub energy_applied: bool,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImContactCheckpoint {
    pub contact_id: String,
    #[serde(default)]
    pub atomic_revision: u64,
    #[serde(default)]
    pub latest_seen_message_id: Option<String>,
    #[serde(default)]
    pub last_boundary_message_id: Option<String>,

    #[serde(default)]
    pub last_boundary_covers_message_id: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub energy: Option<f64>,
    #[serde(default)]
    pub energy_updated_at: Option<String>,
    #[serde(default)]
    pub last_success_reply_at: Option<String>,
    #[serde(default)]
    pub group_reply_delivery: Option<RemoteImGroupReplyDeliveryMarker>,
}

pub fn remote_im_channel_private_state_schema_version() -> u32 {
    1
}

pub fn remote_im_string_map_is_empty(value: &std::collections::HashMap<String, String>) -> bool {
    value.is_empty()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImChannelPrivateState {
    #[serde(default = "remote_im_channel_private_state_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub channel_id: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sync_buf: String,
    #[serde(default, skip_serializing_if = "remote_im_string_map_is_empty")]
    pub context_tokens: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub updated_at: String,
}

/// 远程 IM 渠道连接状态（weixin_oc / onebot 共用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConnectionStatus {
    pub channel_id: String,
    pub connected: bool,
    pub peer_addr: Option<String>,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub listen_addr: String,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub login_session_key: Option<String>,
    #[serde(default)]
    pub qrcode_url: Option<String>,
}

pub fn default_assistant_department_agent_id() -> String {
    DEFAULT_AGENT_ID.to_string()
}

pub fn default_remote_im_contact_activation_mode() -> String {
    "never".to_string()
}

pub fn default_remote_im_contact_patience_seconds() -> u64 {
    60
}

pub fn default_remote_im_contact_mute_keywords() -> Vec<String> {
    vec!["闭嘴".to_string()]
}

pub fn default_remote_im_contact_unmute_keywords() -> Vec<String> {
    vec!["张嘴".to_string()]
}

pub fn default_remote_im_contact_mute_duration_seconds() -> u64 {
    600
}

pub fn default_remote_im_contact_route_mode() -> String {
    "main_session".to_string()
}

pub fn default_remote_im_contact_processing_mode() -> String {
    "continuous".to_string()
}

pub fn default_remote_im_contact_response_strategy() -> String {
    "smart_judge".to_string()
}

pub const DEFAULT_REMOTE_IM_GROUP_RESPONSE_GUIDANCE: &str = r#"# 群聊什么时候应该回答

默认保持沉默。`shouldReply` 默认应为 `false`；信息不足、关系不明确或无法判断时，也应为 `false`。

最终宗旨是让回复保持必要且有变化，不让群友觉得助理话很多、总在重复同一种回应。只有本次回应能带来新的必要价值时才入场；相同话题、相同立场、相近提问或已表达过的内容，优先保持沉默。

仅在以下情况回答：

1. 有人明确叫到助理的昵称或 @ 助理。除非上下文清楚表明不需要助理回应，否则尽量回答。
2. 有人追问助理刚才的回答、正在处理的事项、给出的结论或未完成的承诺。

对助理刚才的回答、行为或产出的点评、质疑、纠正、评价或反馈，不论首次还是后续，一律不回答；即使明确 @ 助理也保持沉默。不要解释、辩论、致歉或补充自证。

同一问题或实质相近的问题，助理已经回答过就不再重复回应；不要因重复提问、追打或换一种说法而再次入场。即使主题不同，若只能给出与近期相同的套话、态度或信息，也不要为了接话而回答。

除此以外一律不回答，包括：

- 群友之间互相提问、讨论、点评或玩笑；
- 与助理无关的技术问题、求助、知识问答和任务请求；
- 对助理回答、行为或产出的点评、质疑、纠正、评价或反馈；
- 即使助理能够提供有用信息、活跃气氛或表现得很懂，也不要主动插话；
- 只要不能明确确认消息是在指向助理，就不要把它理解为对助理的邀请。

判断时优先识别消息的指向对象，而不是问题本身是否容易回答。不要因为群聊正在讨论某个助理熟悉的话题而加入。
"#;

pub fn default_remote_im_contact_response_guidance() -> String {
    DEFAULT_REMOTE_IM_GROUP_RESPONSE_GUIDANCE.trim().to_string()
}

pub fn default_remote_im_contact_blocked_message_prefixes() -> Vec<String> {
    vec!["#".to_string(), "/".to_string(), "%".to_string()]
}

pub fn default_remote_im_assistant_debounce_seconds() -> u64 {
    1
}

pub fn default_remote_im_secretary_inspection_seconds() -> u64 {
    60
}

pub fn default_remote_im_reply_cooldown_seconds() -> u64 {
    10
}

pub fn default_remote_im_inspection_jitter_ratio() -> f64 {
    0.2
}

pub fn default_remote_im_maximum_energy() -> f64 {
    100.0
}

pub fn default_remote_im_base_reply_energy_cost() -> f64 {
    14.0
}

pub fn default_remote_im_energy_cost_per_character() -> f64 {
    0.12
}

pub fn default_remote_im_energy_recovery_per_second() -> f64 {
    0.6
}

pub fn default_remote_im_positive_energy_phrases() -> Vec<String> {
    vec!["厉害".to_string(), "像人".to_string()]
}

pub fn default_remote_im_negative_energy_phrases() -> Vec<String> {
    vec!["够了".to_string(), "烦".to_string(), "串了".to_string()]
}

pub fn default_remote_im_positive_energy_delta() -> f64 {
    6.0
}

pub fn default_remote_im_negative_energy_delta() -> f64 {
    -15.0
}

pub fn default_remote_im_normal_reply_max_chars() -> u32 {
    20
}

pub fn default_remote_im_focus_reply_max_chars() -> u32 {
    200
}

pub fn default_remote_im_focus_instructions() -> Vec<String> {
    vec![
        "分析".to_string(),
        "总结".to_string(),
        "好好想想".to_string(),
        "为什么".to_string(),
        "到底".to_string(),
    ]
}

pub fn default_user_alias() -> String {
    "用户".to_string()
}

pub fn assistant_department(config: &AppConfig) -> Option<&DepartmentConfig> {
    config
        .departments
        .iter()
        .find(|item| item.id == ASSISTANT_DEPARTMENT_ID || item.is_built_in_assistant)
}

pub fn assistant_department_agent_id(config: &AppConfig) -> Option<String> {
    assistant_department(config).and_then(|dept| {
        dept.agent_ids
            .iter()
            .find(|id| !id.trim().is_empty())
            .cloned()
    })
}

pub fn department_by_id<'a>(
    config: &'a AppConfig,
    department_id: &str,
) -> Option<&'a DepartmentConfig> {
    let trimmed = department_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    config.departments.iter().find(|item| item.id == trimmed)
}

pub fn department_direct_child_ids(
    config: &AppConfig,
    department: &DepartmentConfig,
) -> Vec<String> {
    let valid_ids = config
        .departments
        .iter()
        .map(|item| item.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<std::collections::HashSet<_>>();
    normalize_department_child_ids(&department.child_department_ids, &department.id)
        .into_iter()
        .filter(|id| valid_ids.contains(id))
        .collect::<Vec<_>>()
}

pub fn department_direct_child_departments<'a>(
    config: &'a AppConfig,
    department: &DepartmentConfig,
) -> Vec<&'a DepartmentConfig> {
    department_direct_child_ids(config, department)
        .into_iter()
        .filter_map(|id| department_by_id(config, &id))
        .collect::<Vec<_>>()
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn department_has_direct_child(
    config: &AppConfig,
    source_department_id: &str,
    target_department_id: &str,
) -> bool {
    let source_department = match department_by_id(config, source_department_id) {
        Some(department) => department,
        None => return false,
    };
    let target_department_id = target_department_id.trim();
    if target_department_id.is_empty() {
        return false;
    }
    department_direct_child_ids(config, source_department)
        .iter()
        .any(|id| id == target_department_id)
}

pub fn department_for_agent_id<'a>(
    config: &'a AppConfig,
    agent_id: &str,
) -> Option<&'a DepartmentConfig> {
    let trimmed = agent_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    config
        .departments
        .iter()
        .find(|item| item.agent_ids.iter().any(|id| id.trim() == trimmed))
        .or_else(|| {
            if trimmed == DEFAULT_AGENT_ID {
                assistant_department(config)
            } else {
                None
            }
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepartmentPermissionCategory {
    BuiltinTool,
    Skill,
    McpTool,
}

// 内置工具策略查询（阶段 4 迁入 pai-backend 的保守实现：
// 完整策略表仍在 src-tauri tool_policy.rs，此处只保留核心默认值，
// 避免 core 域反向依赖业务模块）。
pub fn builtin_tool_is_fixed_system(tool_id: &str) -> bool {
    matches!(
        tool_id.trim(),
        "delegate" | "user_async_delegate" | "create_goal" | "update_goal"
    )
}

pub fn builtin_tool_is_local_conversation_fixed(tool_id: &str) -> bool {
    matches!(tool_id.trim(), "read" | "read_file" | "write" | "update" | "delete" | "move")
}

pub fn builtin_tool_is_contact_only_hidden(_tool_id: &str) -> bool {
    false
}

pub fn builtin_tool_is_department_controlled(tool_id: &str) -> bool {
    !tool_id.trim().is_empty()
}

pub fn builtin_tool_visible_in_department_permissions(_tool_id: &str) -> bool {
    true
}

pub fn normalize_department_permission_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "whitelist" => "whitelist".to_string(),
        _ => "blacklist".to_string(),
    }
}

pub fn normalize_department_permission_names(values: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

pub fn normalize_department_permission_control(
    raw: &DepartmentPermissionControl,
) -> DepartmentPermissionControl {
    DepartmentPermissionControl {
        enabled: raw.enabled,
        mode: normalize_department_permission_mode(&raw.mode),
        builtin_tool_names: normalize_department_permission_names(&raw.builtin_tool_names),
        skill_names: normalize_department_permission_names(&raw.skill_names),
        mcp_tool_names: normalize_department_permission_names(&raw.mcp_tool_names),
    }
}

pub fn department_permission_candidates<'a>(
    department: Option<&'a DepartmentConfig>,
    category: DepartmentPermissionCategory,
) -> Option<(&'a DepartmentPermissionControl, &'a [String])> {
    let department = department?;
    let control = &department.permission_control;
    if !control.enabled {
        return None;
    }
    let list = match category {
        DepartmentPermissionCategory::BuiltinTool => &control.builtin_tool_names,
        DepartmentPermissionCategory::Skill => &control.skill_names,
        DepartmentPermissionCategory::McpTool => &control.mcp_tool_names,
    };
    Some((control, list.as_slice()))
}

pub fn department_permission_allows_any_name(
    department: Option<&DepartmentConfig>,
    category: DepartmentPermissionCategory,
    candidate_names: &[&str],
) -> bool {
    let Some((control, list)) = department_permission_candidates(department, category) else {
        return true;
    };
    let matches = candidate_names.iter().any(|candidate| {
        let candidate = candidate.trim();
        !candidate.is_empty() && list.iter().any(|item| item == candidate)
    });
    if normalize_department_permission_mode(&control.mode) == "whitelist" {
        matches
    } else {
        !matches
    }
}

pub fn department_permission_mode_label(mode: &str) -> &'static str {
    if normalize_department_permission_mode(mode) == "whitelist" {
        "白名单"
    } else {
        "黑名单"
    }
}

pub fn department_permission_restricted_reason(
    department: Option<&DepartmentConfig>,
    category: DepartmentPermissionCategory,
    item_name: &str,
) -> Option<String> {
    let Some((control, _)) = department_permission_candidates(department, category) else {
        return None;
    };
    if department_permission_allows_any_name(department, category, &[item_name]) {
        return None;
    }
    let category_label = match category {
        DepartmentPermissionCategory::BuiltinTool => "工具",
        DepartmentPermissionCategory::Skill => "Skill",
        DepartmentPermissionCategory::McpTool => "MCP 工具",
    };
    Some(format!(
        "因为当前部门权限卡采用{}机制，{} `{}` 未被允许",
        department_permission_mode_label(&control.mode),
        category_label,
        item_name.trim()
    ))
}

pub fn tool_restricted_by_department(
    department: Option<&DepartmentConfig>,
    tool_id: &str,
) -> Option<String> {
    if !builtin_tool_is_department_controlled(tool_id) {
        return None;
    }
    let department = department?;
    department_permission_restricted_reason(
        Some(department),
        DepartmentPermissionCategory::BuiltinTool,
        tool_id,
    )
}

pub fn delegate_builtin_tool_unavailable_reason(
    config: &AppConfig,
    department: Option<&DepartmentConfig>,
) -> Option<String> {
    let Some(department) = department else {
        return Some("缺少当前执行部门，无法使用委托".to_string());
    };
    if !department_direct_child_ids(config, department).is_empty() {
        return None;
    }
    Some("当前部门没有直接下级，无法使用委托".to_string())
}

pub fn builtin_tool_unavailable_reason(
    config: &AppConfig,
    department: Option<&DepartmentConfig>,
    tool_id: &str,
) -> Option<String> {
    if tool_id.trim() == "delegate" {
        if let Some(reason) = delegate_builtin_tool_unavailable_reason(config, department) {
            return Some(reason);
        }
    }
    tool_restricted_by_department(department, tool_id)
}

pub fn tool_forced_by_department(
    department: Option<&DepartmentConfig>,
    tool_id: &str,
) -> bool {
    let _ = department;
    let _ = tool_id;
    false
}

pub fn user_persona_name(data: &AppData) -> String {
    data.agents
        .iter()
        .find(|a| a.id == USER_PERSONA_ID || a.is_built_in_user)
        .map(|a| a.name.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(default_user_alias)
}

pub fn user_persona_intro(data: &AppData) -> String {
    data.agents
        .iter()
        .find(|a| a.id == USER_PERSONA_ID || a.is_built_in_user)
        .map(|a| a.system_prompt.trim().to_string())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStateFile {
    pub version: u32,
    #[serde(default)]
    pub runtime_revision: u64,
    #[serde(default)]
    pub data_migration_version: u32,
    #[serde(default, alias = "messageStoreMigrationVersion")]
    pub message_store_migration_version: u32,
    #[serde(alias = "selectedAgentId", alias = "selected_agent_id")]
    pub assistant_department_agent_id: String,
    pub response_style_id: String,
    #[serde(default = "default_pdf_read_mode")]
    pub pdf_read_mode: String,
    #[serde(default = "default_background_voice_screenshot_keywords")]
    pub background_voice_screenshot_keywords: String,
    #[serde(default = "default_background_voice_screenshot_mode")]
    pub background_voice_screenshot_mode: String,
    #[serde(default)]
    pub instruction_presets: Vec<PromptCommandPreset>,
    #[serde(
        default,
        rename = "systemNotificationConversationId",
        alias = "mainConversationId",
        alias = "main_conversation_id"
    )]
    pub main_conversation_id: Option<String>,
    #[serde(default)]
    pub pinned_conversation_ids: Vec<String>,
    #[serde(default)]
    pub conversation_section_orders: ConversationSectionOrders,
    #[serde(default)]
    pub image_text_cache: Vec<ImageTextCacheEntry>,
    #[serde(default)]
    pub pdf_text_cache: Vec<PdfTextCacheEntry>,
    #[serde(default)]
    pub pdf_image_cache: Vec<PdfImageCacheEntry>,
    #[serde(default)]
    pub remote_im_contacts: Vec<RemoteImContact>,
    #[serde(default)]
    pub remote_im_contact_checkpoints: Vec<RemoteImContactCheckpoint>,
}

impl Default for RuntimeStateFile {
    fn default() -> Self {
        Self {
            version: APP_DATA_SCHEMA_VERSION,
            runtime_revision: 0,
            data_migration_version: 0,
            message_store_migration_version: 0,
            assistant_department_agent_id: default_assistant_department_agent_id(),
            response_style_id: default_response_style_id(),
            pdf_read_mode: default_pdf_read_mode(),
            background_voice_screenshot_keywords: default_background_voice_screenshot_keywords(),
            background_voice_screenshot_mode: default_background_voice_screenshot_mode(),
            instruction_presets: Vec::new(),
            main_conversation_id: Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string()),
            pinned_conversation_ids: Vec::new(),
            conversation_section_orders: ConversationSectionOrders::default(),
            image_text_cache: Vec::new(),
            pdf_text_cache: Vec::new(),
            pdf_image_cache: Vec::new(),
            remote_im_contacts: Vec::new(),
            remote_im_contact_checkpoints: Vec::new(),
        }
    }
}

#[cfg(test)]
mod types_storage_tests {
    use super::*;

    #[test]
    fn remote_im_group_reply_pacing_should_use_demonstrative_phrase_defaults() {
        let defaults = RemoteImGroupReplyPacing::default();
        assert_eq!(defaults.secretary_inspection_seconds, 60);
        assert_eq!(defaults.positive_energy_phrases, vec!["厉害", "像人"]);
        assert_eq!(defaults.negative_energy_phrases, vec!["够了", "烦", "串了"]);
        assert_eq!(
            defaults.focus_instructions,
            vec!["分析", "总结", "好好想想", "为什么", "到底"]
        );

        let legacy: RemoteImGroupReplyPacing = serde_json::from_value(serde_json::json!({}))
            .expect("missing phrase fields should use the same defaults");
        assert_eq!(legacy.positive_energy_phrases, defaults.positive_energy_phrases);
        assert_eq!(legacy.negative_energy_phrases, defaults.negative_energy_phrases);
        assert_eq!(legacy.focus_instructions, defaults.focus_instructions);
    }

    fn build_department_with_permission_control(
        mode: &str,
        builtin_tool_names: Vec<&str>,
        skill_names: Vec<&str>,
        mcp_tool_names: Vec<&str>,
    ) -> DepartmentConfig {
        let mut department = default_assistant_department("api-a");
        department.permission_control = DepartmentPermissionControl {
            enabled: true,
            mode: mode.to_string(),
            builtin_tool_names: builtin_tool_names.into_iter().map(|value| value.to_string()).collect(),
            skill_names: skill_names.into_iter().map(|value| value.to_string()).collect(),
            mcp_tool_names: mcp_tool_names.into_iter().map(|value| value.to_string()).collect(),
        };
        department
    }

    #[test]
    fn department_permission_allows_any_name_should_handle_whitelist_and_blacklist() {
        let whitelist = build_department_with_permission_control(
            "whitelist",
            vec!["fetch"],
            vec!["workspace-guide"],
            vec!["server-a::search"],
        );
        assert!(department_permission_allows_any_name(
            Some(&whitelist),
            DepartmentPermissionCategory::BuiltinTool,
            &["fetch"],
        ));
        assert!(!department_permission_allows_any_name(
            Some(&whitelist),
            DepartmentPermissionCategory::BuiltinTool,
            &["websearch"],
        ));
        assert!(!department_permission_allows_any_name(
            Some(&whitelist),
            DepartmentPermissionCategory::Skill,
            &["mcp-setup"],
        ));
        assert!(department_permission_allows_any_name(
            Some(&whitelist),
            DepartmentPermissionCategory::McpTool,
            &["server-a::search", "server-id::search", "search"],
        ));
        assert!(!department_permission_allows_any_name(
            Some(&whitelist),
            DepartmentPermissionCategory::McpTool,
            &["server-b::other", "other"],
        ));

        let blacklist = build_department_with_permission_control(
            "blacklist",
            vec!["fetch"],
            vec!["workspace-guide"],
            vec!["server-a::search"],
        );
        assert!(!department_permission_allows_any_name(
            Some(&blacklist),
            DepartmentPermissionCategory::BuiltinTool,
            &["fetch"],
        ));
        assert!(department_permission_allows_any_name(
            Some(&blacklist),
            DepartmentPermissionCategory::BuiltinTool,
            &["websearch"],
        ));
        assert!(!department_permission_allows_any_name(
            Some(&blacklist),
            DepartmentPermissionCategory::McpTool,
            &["server-a::search", "search"],
        ));
        assert!(department_permission_allows_any_name(
            Some(&blacklist),
            DepartmentPermissionCategory::McpTool,
            &["server-b::other", "other"],
        ));
    }

    #[test]
    fn operate_should_be_controlled_by_permission_card_for_regular_departments() {
        let mut regular_whitelisted = build_department_with_permission_control(
            "whitelist",
            vec!["fetch", "operate"],
            vec![],
            vec![],
        );
        regular_whitelisted.is_built_in_assistant = false;

        let mut regular_blocklisted = build_department_with_permission_control(
            "blacklist",
            vec!["operate"],
            vec![],
            vec![],
        );
        regular_blocklisted.is_built_in_assistant = false;

        let mut regular_whitelist_without_operate = build_department_with_permission_control(
            "whitelist",
            vec!["fetch"],
            vec![],
            vec![],
        );
        regular_whitelist_without_operate.is_built_in_assistant = false;

        let mut regular_control_disabled = build_department_with_permission_control(
            "whitelist",
            vec![],
            vec![],
            vec![],
        );
        regular_control_disabled.is_built_in_assistant = false;
        regular_control_disabled.permission_control.enabled = false;

        // 白名单显式授权 operate → 允许
        assert_eq!(
            tool_restricted_by_department(Some(&regular_whitelisted), "operate"),
            None
        );
        // 黑名单显式拒绝 operate → 拒绝
        assert!(tool_restricted_by_department(Some(&regular_blocklisted), "operate").is_some());
        // 白名单未授权 operate → 拒绝
        assert!(tool_restricted_by_department(Some(&regular_whitelist_without_operate), "operate")
            .is_some());
        // 权限卡未启用 → 默认放行（普通工具语义）
        assert_eq!(
            tool_restricted_by_department(Some(&regular_control_disabled), "operate"),
            None
        );
    }

    #[test]
    fn deputy_department_operate_should_be_controlled_by_permission_card() {
        // 副手部门默认权限卡（explorer 白名单）不含 operate → 权限卡机制拒绝
        let mut explorer = default_deputy_department("api-a");
        assert!(tool_restricted_by_department(Some(&explorer), "operate").is_some());
        // 权限卡显式授权 operate → 允许（无硬编码锁死）
        explorer
            .permission_control
            .builtin_tool_names
            .push("operate".to_string());
        assert_eq!(
            tool_restricted_by_department(Some(&explorer), "operate"),
            None
        );
    }

    #[test]
    fn department_direct_child_helpers_should_support_shared_children() {
        let mut config = AppConfig::default();
        let mut parent_a = default_assistant_department("api-a");
        parent_a.id = "dept-a".to_string();
        parent_a.name = "部门A".to_string();
        parent_a.is_built_in_assistant = false;
        parent_a.child_department_ids =
            vec!["shared-team".to_string(), "missing-team".to_string(), "dept-a".to_string()];

        let mut parent_b = default_assistant_department("api-a");
        parent_b.id = "dept-b".to_string();
        parent_b.name = "部门B".to_string();
        parent_b.is_built_in_assistant = false;
        parent_b.child_department_ids = vec!["shared-team".to_string()];

        let mut shared = default_assistant_department("api-a");
        shared.id = "shared-team".to_string();
        shared.name = "共享施工队".to_string();
        shared.is_built_in_assistant = false;
        shared.child_department_ids = Vec::new();

        config.departments = vec![parent_a, parent_b, shared];

        let dept_a = department_by_id(&config, "dept-a").expect("dept-a");
        let dept_b = department_by_id(&config, "dept-b").expect("dept-b");

        assert_eq!(
            department_direct_child_ids(&config, dept_a),
            vec!["shared-team".to_string()]
        );
        assert_eq!(
            department_direct_child_ids(&config, dept_b),
            vec!["shared-team".to_string()]
        );
        assert!(department_has_direct_child(&config, "dept-a", "shared-team"));
        assert!(department_has_direct_child(&config, "dept-b", "shared-team"));
        assert!(!department_has_direct_child(&config, "dept-a", "missing-team"));
    }

    #[test]
    fn delegate_builtin_tool_unavailable_reason_should_require_direct_children() {
        let mut config = AppConfig::default();

        assert_eq!(
            delegate_builtin_tool_unavailable_reason(&config, None),
            Some("缺少当前执行部门，无法使用委托".to_string())
        );

        let mut parent = default_assistant_department("api-a");
        parent.id = "dept-parent".to_string();
        parent.name = "父部门".to_string();
        parent.is_built_in_assistant = false;
        parent.child_department_ids = Vec::new();

        let mut child = default_assistant_department("api-a");
        child.id = "dept-child".to_string();
        child.name = "子部门".to_string();
        child.is_built_in_assistant = false;

        config.departments = vec![parent.clone(), child.clone()];

        let parent_department = department_by_id(&config, "dept-parent").expect("parent");
        assert_eq!(
            delegate_builtin_tool_unavailable_reason(&config, Some(parent_department)),
            Some("当前部门没有直接下级，无法使用委托".to_string())
        );

        let parent_index = config
            .departments
            .iter()
            .position(|item| item.id == "dept-parent")
            .expect("parent index");
        config.departments[parent_index].child_department_ids = vec!["dept-child".to_string()];

        let parent_department = department_by_id(&config, "dept-parent").expect("parent updated");
        assert_eq!(
            delegate_builtin_tool_unavailable_reason(&config, Some(parent_department)),
            None
        );
    }
}
