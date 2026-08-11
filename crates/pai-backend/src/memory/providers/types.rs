use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProviderKind {
    OpenAIEmbedding,
    GeminiEmbedding,
    VllmRerank,
    DeterministicLocal,
}

#[derive(Debug, Clone)]
pub struct MemoryProviderApiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRerankItem {
    pub index: usize,
    pub relevance_score: f64,
}

pub trait MemoryEmbeddingProvider: Send + Sync {
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
}

#[allow(dead_code)]
pub trait MemoryRerankProvider: Send + Sync {
    fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<MemoryRerankItem>, String>;
}

pub fn memory_run_async<F, T>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        // Already inside Tokio runtime: run this sync bridge in a blocking section
        // and drive the future with the current runtime handle.
        return tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future));
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("Create async runtime failed: {err}"))?;
    runtime.block_on(future)
}

pub fn memory_join_url(base_url: &str, suffix: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim().trim_end_matches('/'),
        suffix.trim().trim_start_matches('/'),
    )
}
