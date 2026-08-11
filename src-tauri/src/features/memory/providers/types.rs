#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryProviderKind {
    OpenAIEmbedding,
    GeminiEmbedding,
    VllmRerank,
    DeterministicLocal,
}

#[derive(Debug, Clone)]
pub(crate) struct MemoryProviderApiConfig {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryRerankItem {
    pub(crate) index: usize,
    pub(crate) relevance_score: f64,
}

pub(crate) trait MemoryEmbeddingProvider: Send + Sync {
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
}

#[allow(dead_code)]
pub(crate) trait MemoryRerankProvider: Send + Sync {
    fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<MemoryRerankItem>, String>;
}

pub(crate) fn memory_run_async<F, T>(future: F) -> Result<T, String>
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

pub(crate) fn memory_join_url(base_url: &str, suffix: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim().trim_end_matches('/'),
        suffix.trim().trim_start_matches('/'),
    )
}
