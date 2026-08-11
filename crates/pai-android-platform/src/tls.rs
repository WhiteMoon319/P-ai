//! Android 静态 TLS 根证书（纯逻辑，无平台依赖）。

/// 应用静态 WebPKI 根证书（从 src-tauri android_workspace/rootfs_installer.rs 迁入）。
pub fn android_workspace_apply_static_webpki_roots(
    builder: reqwest::ClientBuilder,
) -> Result<reqwest::ClientBuilder, String> {
    let mut roots = Vec::with_capacity(webpki_root_certs::TLS_SERVER_ROOT_CERTS.len());
    for der in webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter() {
        roots.push(
            reqwest::tls::Certificate::from_der(der.as_ref())
                .map_err(|err| format!("加载 Android 静态 TLS 根证书失败: {err}"))?,
        );
    }
    Ok(builder.tls_certs_only(roots))
}
