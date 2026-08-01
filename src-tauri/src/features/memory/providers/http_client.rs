// ==================== 记忆提供方 HTTP 客户端 ====================
// Android 上 reqwest 默认走 rustls-platform-verifier，未初始化会 panic；
// 统一改用静态 WebPKI 根证书构建，保证嵌入/重排等外部调用在 Android 可用。

pub(crate) fn memory_http_client() -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(15));
    #[cfg(target_os = "android")]
    {
        let mut roots = Vec::with_capacity(webpki_root_certs::TLS_SERVER_ROOT_CERTS.len());
        for der in webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter() {
            roots.push(
                reqwest::tls::Certificate::from_der(der.as_ref())
                    .map_err(|err| format!("加载 Android 静态 TLS 根证书失败: {err}"))?,
            );
        }
        builder = builder.tls_certs_only(roots);
    }
    builder
        .build()
        .map_err(|err| format!("Build memory HTTP client failed: {err}"))
}
