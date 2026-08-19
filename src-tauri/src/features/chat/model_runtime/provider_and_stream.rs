use super::*;
// ---- 共享类型 ----
// types.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::screenshot_cache_types::*;

// ---- 截图缓存基础设施 ----
// screenshot_cache 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::screenshot_cache::*;

// ---- 内置工具统一策略表 ----
// tool_policy 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::tool_policy::*;

// ---- 流式收集 ----
#[path = "provider_and_stream/stream_collect.rs"]
mod provider_stream_collect;
pub(crate) use provider_stream_collect::*;

// ---- 工具组装 ----
#[path = "provider_and_stream/tool_assembly.rs"]
mod provider_tool_assembly;
pub(crate) use provider_tool_assembly::*;

// ---- 统一工具循环 ----
#[path = "provider_and_stream/tool_loop.rs"]
mod provider_tool_loop;
pub(crate) use provider_tool_loop::*;

// ---- OpenAI provider ----
#[path = "provider_and_stream/openai_style.rs"]
mod provider_openai_style;
pub(crate) use provider_openai_style::*;

// ---- Gemini provider ----
#[path = "provider_and_stream/gemini.rs"]
mod provider_gemini;
pub(crate) use provider_gemini::*;

// ---- Anthropic provider ----
#[path = "provider_and_stream/anthropic.rs"]
mod provider_anthropic;
pub(crate) use provider_anthropic::*;

// ---- 路由分发 + 日志 ----
#[path = "provider_and_stream/router.rs"]
mod provider_router;
pub(crate) use provider_router::*;

// ---- Vision API ----
#[path = "provider_and_stream/vision.rs"]
mod provider_vision;
pub(crate) use provider_vision::*;
