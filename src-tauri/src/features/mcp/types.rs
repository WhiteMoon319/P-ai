use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
)]
pub(crate) enum McpTransportKind {
    Stdio,
    StreamableHttp,
    Sse,
}

impl McpTransportKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::StreamableHttp => "streamable_http",
            Self::Sse => "sse",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct ParsedMcpServerDefinition {
    pub(crate) transport: McpTransportKind,
    pub(crate) command: Option<String>,
    pub(crate) args: Vec<String>,
    pub(crate) env: std::collections::HashMap<String, String>,
    pub(crate) cwd: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) bearer_token_env_var: Option<String>,
    pub(crate) http_headers: std::collections::HashMap<String, String>,
    pub(crate) env_http_headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) definition_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerIdInput {
    pub(crate) server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpDefinitionValidateInput {
    pub(crate) definition_json: String,
    /// 全工作区其他卡片的组内成员名集合，用于跨卡片重名检测
    #[serde(default)]
    pub(crate) existing_member_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpDefinitionValidateResult {
    pub(crate) ok: bool,
    pub(crate) transport: Option<String>,
    pub(crate) server_name: Option<String>,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) schema_version: Option<String>,
    #[serde(default)]
    pub(crate) error_code: Option<String>,
    #[serde(default)]
    pub(crate) details: Vec<String>,
    #[serde(default)]
    pub(crate) issues: Vec<McpValidationIssue>,
    #[serde(default)]
    pub(crate) migrated_definition_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpFixDefinitionInput {
    pub(crate) definition_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpFixDefinitionResult {
    pub(crate) ok: bool,
    #[serde(default)]
    pub(crate) fixed_definition_json: Option<String>,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) issues: Vec<McpValidationIssue>,
    #[serde(default)]
    pub(crate) model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpToolDescriptor {
    pub(crate) tool_name: String,
    pub(crate) description: String,
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) compatibility_error: Option<String>,
    #[serde(default)]
    pub(crate) member_name: String,
    #[serde(default)]
    pub(crate) raw_tool_name: String,
    #[serde(default)]
    pub(crate) parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpListServerToolsResult {
    pub(crate) server_id: String,
    pub(crate) tools: Vec<McpToolDescriptor>,
    pub(crate) elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpSetToolEnabledInput {
    pub(crate) server_id: String,
    pub(crate) tool_name: String,
    pub(crate) enabled: bool,
}
