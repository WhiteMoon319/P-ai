use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use serde::{Deserialize, Serialize};

use crate::mcp::parser::McpValidationIssue;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
)]
pub enum McpTransportKind {
    Stdio,
    StreamableHttp,
    Sse,
}

impl McpTransportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::StreamableHttp => "streamable_http",
            Self::Sse => "sse",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParsedMcpServerDefinition {
    pub transport: McpTransportKind,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub bearer_token_env_var: Option<String>,
    pub http_headers: std::collections::HashMap<String, String>,
    pub env_http_headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInput {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub definition_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerIdInput {
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDefinitionValidateInput {
    pub definition_json: String,
    /// 全工作区其他卡片的组内成员名集合，用于跨卡片重名检测
    #[serde(default)]
    pub existing_member_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDefinitionValidateResult {
    pub ok: bool,
    pub transport: Option<String>,
    pub server_name: Option<String>,
    pub message: String,
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub details: Vec<String>,
    #[serde(default)]
    pub issues: Vec<McpValidationIssue>,
    #[serde(default)]
    pub migrated_definition_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpFixDefinitionInput {
    pub definition_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpFixDefinitionResult {
    pub ok: bool,
    #[serde(default)]
    pub fixed_definition_json: Option<String>,
    pub message: String,
    #[serde(default)]
    pub issues: Vec<McpValidationIssue>,
    #[serde(default)]
    pub model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDescriptor {
    pub tool_name: String,
    pub description: String,
    pub enabled: bool,
    #[serde(default)]
    pub compatibility_error: Option<String>,
    #[serde(default)]
    pub member_name: String,
    #[serde(default)]
    pub raw_tool_name: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpListServerToolsResult {
    pub server_id: String,
    pub tools: Vec<McpToolDescriptor>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSetToolEnabledInput {
    pub server_id: String,
    pub tool_name: String,
    pub enabled: bool,
}
