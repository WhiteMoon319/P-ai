use super::*;
use uuid::Uuid;

pub(crate) fn resolve_api_config(
    app_config: &AppConfig,
    requested_id: Option<&str>,
) -> Result<ResolvedApiConfig, String> {
    if let Some(debug_cfg) = read_debug_api_config()? {
        let enabled = debug_cfg.enabled.unwrap_or(true);
        let request_format_ok = debug_cfg
            .request_format
            .unwrap_or(RequestFormat::OpenAI)
            .is_openai_style();

        if enabled && request_format_ok {
            if debug_cfg.api_key.trim().is_empty() {
                return Err(".debug/api-key.json exists but apiKey is empty.".to_string());
            }
            return Ok(ResolvedApiConfig {
                provider_id: None,
                provider_api_keys: Vec::new(),
                provider_key_cursor: 0,
                request_format: RequestFormat::OpenAI,
                allow_concurrent_requests: false,
                max_concurrent_requests: None,
                base_url: debug_cfg.base_url.trim().to_string(),
                api_key: debug_cfg.api_key.trim().to_string(),
                model: debug_cfg.model.trim().to_string(),
                reasoning_effort: None,
                temperature: debug_cfg.temperature.map(|value| value.clamp(0.0, 2.0)),
                max_output_tokens: None,
                prompt_cache_key: None,
                extra_headers: Vec::new(),
                codex_auth: None,
                codex_custom_api_key: None,
            });
        }
    }

    let selected = resolve_selected_api_config(app_config, requested_id).ok_or_else(|| {
        "No API config configured. Please add at least one API config.".to_string()
    })?;

    let selected_provider_id = parse_api_endpoint_id(&selected.id)
        .and_then(|(provider_id, _model_id)| {
            app_config
                .api_providers
                .iter()
                .any(|provider| provider.id == provider_id)
                .then_some(provider_id)
        });
    let selected_provider = selected_provider_id.as_deref().and_then(|provider_id| {
        app_config
            .api_providers
            .iter()
            .find(|provider| provider.id == provider_id)
    });
    let mut selected_api_key = selected_provider
        .map(peek_provider_api_key)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| selected.api_key.trim().to_string());
    let mut extra_headers = Vec::<(String, String)>::new();
    let mut codex_auth = None;
    if selected.request_format.is_codex() {
        let provider = selected_provider.ok_or_else(|| {
            "Codex provider not found. Please save the provider config first.".to_string()
        })?;
        let normalized_mode = normalize_codex_auth_mode(&provider.codex_auth_mode);
        match normalized_mode.as_str() {
            CODEX_AUTH_MODE_CUSTOM_URL => {
                selected_api_key = provider
                    .codex_custom_api_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("")
                    .to_string();
            }
            CODEX_AUTH_MODE_MANAGED_OAUTH | CODEX_AUTH_MODE_READ_LOCAL => {
                let resolved = read_codex_runtime_auth_snapshot(
                    &provider.id,
                    normalized_mode.as_str(),
                    &provider.codex_local_auth_path,
                )?;
                if let Some(account_id) = resolved
                    .account_id
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    extra_headers.push((
                        "ChatGPT-Account-Id".to_string(),
                        account_id.to_string(),
                    ));
                }
                selected_api_key = resolved.access_token.clone();
                codex_auth = Some(resolved);
            }
            _ => {}
        }
        extra_headers.push(("Session-Id".to_string(), Uuid::new_v4().to_string()));
    }

    if selected_api_key.trim().is_empty() {
        return Err("Selected API config API key is empty. Please fill it in settings.".to_string());
    }

    Ok(ResolvedApiConfig {
        provider_id: selected_provider_id,
        provider_api_keys: selected_provider
            .map(|provider| provider.api_keys.clone())
            .unwrap_or_default(),
        provider_key_cursor: selected_provider
            .map(|provider| provider.key_cursor as usize)
            .unwrap_or(0),
        request_format: selected.request_format,
        allow_concurrent_requests: selected_provider
            .map(|provider| provider.allow_concurrent_requests)
            .unwrap_or(selected.allow_concurrent_requests),
        max_concurrent_requests: selected_provider
            .and_then(|provider| provider.max_concurrent_requests)
            .or(selected.max_concurrent_requests),
        base_url: selected_provider
            .map(|provider| {
                if selected.request_format.is_codex()
                    && normalize_codex_auth_mode(&provider.codex_auth_mode)
                        == CODEX_AUTH_MODE_CUSTOM_URL
                {
                    provider
                        .codex_custom_url
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or(selected.base_url.trim())
                        .to_string()
                } else {
                    selected.base_url.trim().to_string()
                }
            })
            .unwrap_or_else(|| selected.base_url.trim().to_string()),
        api_key: selected_api_key,
        model: selected.model.trim().to_string(),
        reasoning_effort: selected_reasoning_effort_for_runtime(&selected),
        temperature: selected
            .custom_temperature_enabled
            .then_some(selected.temperature.clamp(0.0, 2.0))
            .filter(|_| !selected.request_format.is_codex()),
        max_output_tokens: (selected.request_format.is_anthropic()
            || selected.custom_max_output_tokens_enabled)
            .then_some(selected.max_output_tokens)
            .filter(|_| !selected.request_format.is_codex()),
        prompt_cache_key: None,
        extra_headers,
        codex_auth,
        codex_custom_api_key: selected.codex_custom_api_key.clone(),
    })
}
