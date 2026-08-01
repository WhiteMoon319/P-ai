fn ide_context_generate_bridge_token() -> String {
    Uuid::new_v4().to_string()
}

fn ide_context_generate_remote_password() -> String {
    generate_web_access_password()
}

fn ide_context_normalize_remote_password(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_uppercase())
        .collect()
}

fn ide_context_remote_password(runtime: &IdeContextRuntime) -> Result<String, String> {
    let auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
    Ok(auth.remote_password.clone())
}

fn ide_context_effective_remote_password(
    state: &AppState,
    runtime: &IdeContextRuntime,
) -> Result<String, String> {
    let config = state_read_config_cached(state)?;
    let password = normalize_web_access_password(&config.web_access_password);
    if !password.trim().is_empty() {
        return Ok(password);
    }
    ide_context_remote_password(runtime)
}

fn ide_context_current_port(runtime: &IdeContextRuntime) -> Option<u16> {
    runtime.current_port.lock().ok().and_then(|guard| *guard)
}

fn ide_context_set_current_port(runtime: &IdeContextRuntime, port: Option<u16>) {
    if let Ok(mut slot) = runtime.current_port.lock() {
        *slot = port;
    }
}

fn ide_context_verify_remote_password(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
    provided: &str,
) -> Result<bool, String> {
    let expected = match state {
        Some(state) => ide_context_effective_remote_password(state, runtime)?,
        None => ide_context_remote_password(runtime)?,
    };
    let provided = ide_context_normalize_remote_password(provided);
    if provided.is_empty() {
        return Ok(false);
    }
    Ok(provided == ide_context_normalize_remote_password(&expected))
}

fn ide_context_peer_is_local(peer_addr: &std::net::SocketAddr) -> bool {
    peer_addr.ip().is_loopback()
}

fn ide_context_ws_header_value(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ide_context_ws_request_host_matches(request: &Request, origin_host: &str, port: u16) -> bool {
    let Some(raw_host) = ide_context_ws_header_value(request, "host") else {
        return false;
    };
    let host_url = format!("http://{raw_host}");
    let Ok(parsed) = reqwest::Url::parse(&host_url) else {
        return false;
    };
    if parsed.port_or_known_default() != Some(port) {
        return false;
    }
    parsed
        .host_str()
        .map(|host| host.eq_ignore_ascii_case(origin_host))
        .unwrap_or(false)
}

fn ide_context_ws_origin_allowed(request: &Request, port: u16) -> bool {
    let Some(origin) = ide_context_ws_header_value(request, "origin") else {
        return true;
    };
    if origin.starts_with("vscode-webview://") {
        return true;
    }
    let Ok(parsed) = reqwest::Url::parse(&origin) else {
        return false;
    };
    if parsed.scheme() != "http" || parsed.port_or_known_default() != Some(port) {
        return false;
    }
    parsed
        .host_str()
        .map(|host| ide_context_ws_request_host_matches(request, host, port))
        .unwrap_or(false)
}

fn ide_context_ws_forbidden_response(message: &str) -> tokio_tungstenite::tungstenite::handshake::server::ErrorResponse {
    let mut response =
        tokio_tungstenite::tungstenite::handshake::server::ErrorResponse::new(Some(message.to_string()));
    *response.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
    response
}

#[derive(Debug, Clone)]
struct IdeContextLanHostCandidate {
    ip: std::net::Ipv4Addr,
    adapter_name: String,
    adapter_description: String,
    has_gateway: bool,
    active: bool,
}

fn ide_context_ipv4_in_cidr(ip: std::net::Ipv4Addr, network: [u8; 4], prefix_len: u8) -> bool {
    let ip_num = u32::from(ip);
    let network_num = u32::from(std::net::Ipv4Addr::from(network));
    let mask = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix_len))
    };
    (ip_num & mask) == (network_num & mask)
}

fn ide_context_ipv4_is_private_lan(ip: std::net::Ipv4Addr) -> bool {
    ip.is_private()
}

fn ide_context_ipv4_is_remote_link_candidate(ip: std::net::Ipv4Addr) -> bool {
    !ip.is_loopback()
        && !ip.is_unspecified()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && !ip.is_broadcast()
        && !ide_context_ipv4_in_cidr(ip, [198, 18, 0, 0], 15)
        && !ide_context_ipv4_in_cidr(ip, [100, 64, 0, 0], 10)
        && !ide_context_ipv4_in_cidr(ip, [192, 0, 2, 0], 24)
        && !ide_context_ipv4_in_cidr(ip, [198, 51, 100, 0], 24)
        && !ide_context_ipv4_in_cidr(ip, [203, 0, 113, 0], 24)
        && ide_context_ipv4_is_private_lan(ip)
}

fn ide_context_adapter_name_is_virtual(name: &str, description: &str) -> bool {
    let text = format!("{name} {description}").to_ascii_lowercase();
    [
        "mihomo",
        "clash",
        "tun",
        "tap",
        "wintun",
        "wireguard",
        "tailscale",
        "zerotier",
        "vethernet",
        "hyper-v",
        "wsl",
        "vmware",
        "virtualbox",
        "docker",
        "loopback",
        "bluetooth",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn ide_context_lan_host_rank(candidate: &IdeContextLanHostCandidate) -> (u8, u8, u8, u32) {
    let virtual_adapter = ide_context_adapter_name_is_virtual(
        &candidate.adapter_name,
        &candidate.adapter_description,
    );
    (
        if virtual_adapter { 1 } else { 0 },
        if candidate.has_gateway { 0 } else { 1 },
        if candidate.active { 0 } else { 1 },
        u32::from(candidate.ip),
    )
}

fn ide_context_collect_default_route_lan_host() -> Vec<IdeContextLanHostCandidate> {
    let mut hosts = Vec::new();
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        let _ = socket.connect("8.8.8.8:80");
        if let Ok(addr) = socket.local_addr() {
            if let std::net::IpAddr::V4(ip) = addr.ip() {
                hosts.push(IdeContextLanHostCandidate {
                    ip,
                    adapter_name: "default-route".to_string(),
                    adapter_description: String::new(),
                    has_gateway: true,
                    active: true,
                });
            }
        }
    }
    hosts
}

fn ide_context_json_strings(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::String(text)) => vec![text.trim().to_string()],
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(|text| text.trim().to_string()))
            .filter(|text| !text.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn ide_context_parse_windows_lan_host_candidates(
    value: serde_json::Value,
) -> Vec<IdeContextLanHostCandidate> {
    let entries = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let adapter_name = object
            .get("InterfaceAlias")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let adapter_description = object
            .get("InterfaceDescription")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let active = object
            .get("Status")
            .and_then(|value| value.as_str())
            .map(|value| value.eq_ignore_ascii_case("up"))
            .unwrap_or(true);
        let has_gateway = !ide_context_json_strings(object.get("IPv4DefaultGateway")).is_empty();
        for ip_text in ide_context_json_strings(object.get("IPv4Address")) {
            if let Ok(ip) = ip_text.parse::<std::net::Ipv4Addr>() {
                candidates.push(IdeContextLanHostCandidate {
                    ip,
                    adapter_name: adapter_name.clone(),
                    adapter_description: adapter_description.clone(),
                    has_gateway,
                    active,
                });
            }
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
fn ide_context_collect_windows_lan_hosts() -> Vec<IdeContextLanHostCandidate> {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
Get-NetIPConfiguration | ForEach-Object {
  [pscustomobject]@{
    InterfaceAlias = $_.InterfaceAlias
    InterfaceDescription = $_.InterfaceDescription
    Status = $_.NetAdapter.Status
    IPv4Address = @($_.IPv4Address | ForEach-Object { $_.IPAddress })
    IPv4DefaultGateway = @($_.IPv4DefaultGateway | ForEach-Object { $_.NextHop })
  }
} | ConvertTo-Json -Depth 5 -Compress
"#;
    let mut command = std::process::Command::new("powershell");
    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        // PowerShell 是控制台程序，后台探测不能让 GUI 应用弹出控制台窗口。
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = command.output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => ide_context_parse_windows_lan_host_candidates(value),
        Err(_) => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn ide_context_collect_windows_lan_hosts() -> Vec<IdeContextLanHostCandidate> {
    Vec::new()
}

fn ide_context_lan_hosts() -> Vec<String> {
    let mut candidates = ide_context_collect_windows_lan_hosts();
    if candidates.is_empty() {
        candidates = ide_context_collect_default_route_lan_host();
    }
    candidates.retain(|candidate| ide_context_ipv4_is_remote_link_candidate(candidate.ip));
    candidates.sort_by_key(ide_context_lan_host_rank);
    let mut seen = std::collections::HashSet::<String>::new();
    candidates
        .into_iter()
        .map(|candidate| candidate.ip.to_string())
        .filter(|host| seen.insert(host.clone()))
        .collect::<Vec<_>>()
}
