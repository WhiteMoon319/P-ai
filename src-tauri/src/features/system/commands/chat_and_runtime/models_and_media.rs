include!("models_catalog.rs");
include!("attachments_io.rs");
include!("attachment_transfer.rs");
include!("attachment_transfer_web.rs");
#[cfg(test)]
mod attachment_transfer_tests {
    include!("attachment_transfer_tests.rs");
}
include!("stt_transcribe.rs");
include!("model_providers.rs");
include!("aliyun_multimodal_cache.rs");
include!("tools_and_cache.rs");
include!("tool_catalog.rs");
