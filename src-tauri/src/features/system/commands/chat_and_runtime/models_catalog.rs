async fn fetch_models_openai(input: &RefreshModelsInput) -> Result<Vec<String>, String> {
    let base = input.base_url.trim().trim_end_matches('/');
    let api_key = input.api_key.trim();

    if api_key.contains('\r') || api_key.contains('\n') {
        return Err("API key contains newline characters. Please paste a single-line token.".to_string());
    }
    if matches!(api_key, "..." | "***" | "•••" | "···") {
        return Err("API key is still a placeholder ('...' / '***'). Please paste the real token.".to_string());
    }
    let auth = format!("Bearer {api_key}");
    let auth_value = HeaderValue::from_str(&auth)
        .map_err(|err| {
            format!(
                "Build authorization header failed: {err}. The API key may contain invalid characters."
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| format!("Build HTTP client failed: {err}"))?;

    let mut urls = vec![format!("{base}/models")];
    if !base.to_ascii_lowercase().ends_with("/v1") {
        urls.push(format!("{base}/v1/models"));
    }
    urls.dedup();

    let mut last_error = String::new();
    for url in urls {
        let resp = client
            .get(&url)
            .header(AUTHORIZATION, auth_value.clone())
            .send()
            .await
            .map_err(|err| format!("Fetch model list failed ({url}): {err}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let raw = resp.text().await.unwrap_or_default();
            let snippet = raw.chars().take(600).collect::<String>();
            last_error = format!("Fetch model list failed: {url} -> {status} | {snippet}");
            if status.as_u16() == 404 {
                continue;
            }
            return Err(last_error);
        }

        let body = resp
            .json::<OpenAIModelListResponse>()
            .await
            .map_err(|err| format!("Parse model list failed ({url}): {err}"))?;

        let mut models = body.data.into_iter().map(|item| item.id).collect::<Vec<_>>();
        models.sort();
        models.dedup();
        return Ok(models);
    }

    Err(last_error)
}
