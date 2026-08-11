pub const APP_DATA_SCHEMA_VERSION: u32 = 1;

// ========== 数据迁移版本门禁 ==========
//
// 版本语义：
//   - DATA_MIGRATION_CURRENT_VERSION：当前数据迁移版本，启动期写回 runtime_state。
//   - V2/V3：data_migration_steps() 注册的版本化迁移步骤，按版本号递增执行。
//
// 新增迁移（v2+）的接入流程：
//   1. 在 app_data_layout.rs 的 data_migration_steps() 注册一个 DataMigrationStep；
//   2. 在此处新增 DATA_MIGRATION_VERSION_V2 常量，并把 CURRENT_VERSION 提到它。
pub const DATA_MIGRATION_VERSION_V1_BASELINE: u32 = 1;
pub const DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES: u32 = 2;
pub const DATA_MIGRATION_VERSION_V3_CHAT_METADATA_SQLITE: u32 = 3;
pub const DATA_MIGRATION_CURRENT_VERSION: u32 = DATA_MIGRATION_VERSION_V3_CHAT_METADATA_SQLITE;
pub const MAX_MULTIMODAL_BYTES: usize = 10 * 1024 * 1024;
pub const DEFAULT_AGENT_ID: &str = "default-agent";
pub const DEPUTY_AGENT_ID: &str = "deputy-agent";
pub const USER_PERSONA_ID: &str = "user-persona";
pub const SYSTEM_PERSONA_ID: &str = "system-persona";
pub const ASSISTANT_DEPARTMENT_ID: &str = "assistant-department";
pub const LEADER_DEPARTMENT_ID: &str = "leader-department";
pub const DEPUTY_DEPARTMENT_ID: &str = "deputy-department";
pub const REVIEWER_DEPARTMENT_ID: &str = "reviewer-department";
pub const SADDLER_DEPARTMENT_ID: &str = "saddler-department";
pub const REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID: &str = "remote-customer-service-department";
pub const DELEGATE_TOOL_KIND_DELEGATE: &str = "delegate";
pub const DELEGATE_TOOL_KIND_USER_MENTION: &str = "user_async_delegate";
pub const SYSTEM_NOTIFICATION_CONVERSATION_ID: &str = "system-notification-conversation";
pub const CONVERSATION_KIND_CHAT: &str = "chat";
pub const CONVERSATION_KIND_SIDE_CHAT: &str = "side_chat";
pub const CONVERSATION_KIND_SYSTEM_NOTIFICATION: &str = "system_notification";
pub const CONVERSATION_KIND_DELEGATE: &str = "delegate";
pub const CONVERSATION_KIND_REMOTE_IM_CONTACT: &str = "remote_im_contact";
pub const DEFAULT_RESPONSE_STYLE_ID: &str = "concise";
pub const DEFAULT_PDF_READ_MODE: &str = "image";
pub const DEFAULT_BACKGROUND_VOICE_SCREENSHOT_MODE: &str = "focused_window";
pub const CHAT_ABORTED_BY_USER_ERROR: &str = "CHAT_ABORTED_BY_USER";
pub const CHAT_DISPATCH_RESTART_AFTER_COMPACTION: &str = "CHAT_DISPATCH_RESTART_AFTER_COMPACTION";
pub const APP_HTTP_ORIGINATOR: &str = "p_ai_desktop";
