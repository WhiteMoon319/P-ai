pub(crate) const APP_DATA_SCHEMA_VERSION: u32 = 1;

// ========== 数据迁移版本门禁 ==========
//
// 版本语义：
//   - DATA_MIGRATION_CURRENT_VERSION：当前数据迁移版本，启动期写回 runtime_state。
//   - V2/V3：data_migration_steps() 注册的版本化迁移步骤，按版本号递增执行。
//
// 新增迁移（v2+）的接入流程：
//   1. 在 app_data_layout.rs 的 data_migration_steps() 注册一个 DataMigrationStep；
//   2. 在此处新增 DATA_MIGRATION_VERSION_V2 常量，并把 CURRENT_VERSION 提到它。
pub(crate) const DATA_MIGRATION_VERSION_V1_BASELINE: u32 = 1;
pub(crate) const DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES: u32 = 2;
pub(crate) const DATA_MIGRATION_VERSION_V3_CHAT_METADATA_SQLITE: u32 = 3;
pub(crate) const DATA_MIGRATION_CURRENT_VERSION: u32 = DATA_MIGRATION_VERSION_V3_CHAT_METADATA_SQLITE;
pub(crate) const MAX_MULTIMODAL_BYTES: usize = 10 * 1024 * 1024;
pub(crate) const DEFAULT_AGENT_ID: &str = "default-agent";
pub(crate) const DEPUTY_AGENT_ID: &str = "deputy-agent";
pub(crate) const USER_PERSONA_ID: &str = "user-persona";
pub(crate) const SYSTEM_PERSONA_ID: &str = "system-persona";
pub(crate) const ASSISTANT_DEPARTMENT_ID: &str = "assistant-department";
pub(crate) const LEADER_DEPARTMENT_ID: &str = "leader-department";
pub(crate) const DEPUTY_DEPARTMENT_ID: &str = "deputy-department";
pub(crate) const REVIEWER_DEPARTMENT_ID: &str = "reviewer-department";
pub(crate) const SADDLER_DEPARTMENT_ID: &str = "saddler-department";
pub(crate) const REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID: &str = "remote-customer-service-department";
pub(crate) const DELEGATE_TOOL_KIND_DELEGATE: &str = "delegate";
pub(crate) const DELEGATE_TOOL_KIND_USER_MENTION: &str = "user_async_delegate";
pub(crate) const SYSTEM_NOTIFICATION_CONVERSATION_ID: &str = "system-notification-conversation";
pub(crate) const CONVERSATION_KIND_CHAT: &str = "chat";
pub(crate) const CONVERSATION_KIND_SIDE_CHAT: &str = "side_chat";
pub(crate) const CONVERSATION_KIND_SYSTEM_NOTIFICATION: &str = "system_notification";
pub(crate) const CONVERSATION_KIND_DELEGATE: &str = "delegate";
pub(crate) const CONVERSATION_KIND_REMOTE_IM_CONTACT: &str = "remote_im_contact";
pub(crate) const DEFAULT_RESPONSE_STYLE_ID: &str = "concise";
pub(crate) const DEFAULT_PDF_READ_MODE: &str = "image";
pub(crate) const DEFAULT_BACKGROUND_VOICE_SCREENSHOT_MODE: &str = "focused_window";
pub(crate) const CHAT_ABORTED_BY_USER_ERROR: &str = "CHAT_ABORTED_BY_USER";
pub(crate) const CHAT_DISPATCH_RESTART_AFTER_COMPACTION: &str = "CHAT_DISPATCH_RESTART_AFTER_COMPACTION";
pub(crate) const APP_HTTP_ORIGINATOR: &str = "p_ai_desktop";
