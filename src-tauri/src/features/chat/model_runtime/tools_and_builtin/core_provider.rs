// ========== provider 调用 ==========
// core_provider_gemini 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::core_provider_gemini::*;
include!("core_provider_calls.rs");

// ========== provider 通用工具与错误 ==========
// core_provider_utils 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::core_provider_utils::*;
