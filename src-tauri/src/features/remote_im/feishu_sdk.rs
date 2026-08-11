pub(crate) struct FeishuSdk;

impl FeishuSdk {
    pub(crate) async fn tenant_access_token(&self, channel: &RemoteImChannelConfig) -> Result<String, String> {
        let started = std::time::Instant::now();
        remote_im_log(
            "INFO",
            "feishu.tenant_access_token",
            serde_json::json!({
                "channel_id": channel.id,
                "status": "开始"
            }),
        );
        let app_id = remote_im_credential_text(&channel.credentials, "appId");
        let app_secret = remote_im_credential_text(&channel.credentials, "appSecret");
        if app_id.is_empty() || app_secret.is_empty() {
            let err = format!("feishu channel '{}' missing appId/appSecret", channel.id);
            remote_im_log(
                "ERROR",
                "feishu.tenant_access_token",
                serde_json::json!({
                    "channel_id": channel.id,
                    "status": "失败",
                    "error": err
                }),
            );
            return Err(err);
        }
        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12));
        #[cfg(target_os = "android")]
        {
            client_builder = features_system_commands::android_workspace_rootfs_installer::android_workspace_apply_static_webpki_roots(client_builder)?;
        }
        let client = client_builder
            .build()
            .map_err(|err| {
                let msg = format!("build feishu client failed: {err}");
                remote_im_log(
                    "ERROR",
                    "feishu.tenant_access_token",
                    serde_json::json!({
                        "channel_id": channel.id,
                        "status": "失败",
                        "error": msg
                    }),
                );
                msg
            })?;
        let response = client
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&serde_json::json!({
                "app_id": app_id,
                "app_secret": app_secret
            }))
            .send()
            .await
            .map_err(|err| {
                let msg = format!("feishu token request failed: {err}");
                remote_im_log(
                    "ERROR",
                    "feishu.tenant_access_token",
                    serde_json::json!({
                        "channel_id": channel.id,
                        "status": "失败",
                        "error": msg
                    }),
                );
                msg
            })?;
        let body = response
            .json::<Value>()
            .await
            .map_err(|err| {
                let msg = format!("parse feishu token response failed: {err}");
                remote_im_log(
                    "ERROR",
                    "feishu.tenant_access_token",
                    serde_json::json!({
                        "channel_id": channel.id,
                        "status": "失败",
                        "error": msg
                    }),
                );
                msg
            })?;
        let token = body
            .get("tenant_access_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if token.is_empty() {
            let err = format!("feishu token missing: {}", body);
            remote_im_log(
                "ERROR",
                "feishu.tenant_access_token",
                serde_json::json!({
                    "channel_id": channel.id,
                    "status": "失败",
                    "error": err
                }),
            );
            return Err(err);
        }
        remote_im_log(
            "INFO",
            "feishu.tenant_access_token",
            serde_json::json!({
                "channel_id": channel.id,
                "status": "完成",
                "duration_ms": started.elapsed().as_millis(),
                "token_masked": format!("***len:{}***", token.len())
            }),
        );
        Ok(token.to_string())
    }

    pub(crate) async fn send_message(
        &self,
        client: &reqwest::Client,
        token: &str,
        receive_id_type: &str,
        receive_id: &str,
        msg_type: &str,
        content_obj: Value,
    ) -> Result<String, RemoteImSdkSendError> {
        let response = client
            .post(format!(
                "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type={receive_id_type}"
            ))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({
                "receive_id": receive_id,
                "msg_type": msg_type,
                "content": content_obj.to_string()
            }))
            .send()
            .await
            .map_err(|err| {
                RemoteImSdkSendError::uncertain(format!("feishu send failed: {err}"))
            })?;
        let status = response.status();
        let body = response
            .json::<Value>()
            .await
            .map_err(|err| {
                RemoteImSdkSendError::uncertain(format!(
                    "parse feishu send response failed: {err}"
                ))
            })?;
        if !status.is_success() {
            return Err(remote_im_http_rejection_error(
                status,
                format!("feishu send rejected http {}: {}", status.as_u16(), body),
            ));
        }
        if body.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
            return Err(RemoteImSdkSendError::definitely_not_sent(format!(
                "feishu send rejected: {}",
                body
            )));
        }
        Ok(body
            .get("data")
            .and_then(|v| v.get("message_id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    pub(crate) async fn upload_image_key(
        &self,
        client: &reqwest::Client,
        token: &str,
        image_name: &str,
        raw: Vec<u8>,
    ) -> Result<String, String> {
        let part = reqwest::multipart::Part::bytes(raw).file_name(image_name.to_string());
        let form = reqwest::multipart::Form::new()
            .text("image_type", "message")
            .part("image", part);
        let response = client
            .post("https://open.feishu.cn/open-apis/im/v1/images")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .multipart(form)
            .send()
            .await
            .map_err(|err| format!("feishu upload image failed: {err}"))?;
        let body = response
            .json::<Value>()
            .await
            .map_err(|err| format!("parse feishu upload image response failed: {err}"))?;
        if body.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
            return Err(format!("feishu upload image rejected: {}", body));
        }
        let image_key = body
            .get("data")
            .and_then(|v| v.get("image_key"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if image_key.is_empty() {
            return Err(format!("feishu upload image missing image_key: {}", body));
        }
        Ok(image_key)
    }

    pub(crate) async fn upload_file_key(
        &self,
        client: &reqwest::Client,
        token: &str,
        file_name: &str,
        mime: &str,
        raw: Vec<u8>,
    ) -> Result<String, String> {
        let part = reqwest::multipart::Part::bytes(raw)
            .file_name(file_name.to_string())
            .mime_str(mime)
            .map_err(|err| format!("build feishu file part mime failed: {err}"))?;
        let form = reqwest::multipart::Form::new()
            .text("file_type", "stream")
            .text("file_name", file_name.to_string())
            .part("file", part);
        let response = client
            .post("https://open.feishu.cn/open-apis/im/v1/files")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .multipart(form)
            .send()
            .await
            .map_err(|err| format!("feishu upload file failed: {err}"))?;
        let body = response
            .json::<Value>()
            .await
            .map_err(|err| format!("parse feishu upload file response failed: {err}"))?;
        if body.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
            return Err(format!("feishu upload file rejected: {}", body));
        }
        let file_key = body
            .get("data")
            .and_then(|v| v.get("file_key"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if file_key.is_empty() {
            return Err(format!("feishu upload file missing file_key: {}", body));
        }
        Ok(file_key)
    }
}

impl RemoteImSdk for FeishuSdk {
    fn platform(&self) -> RemoteImPlatform {
        RemoteImPlatform::Feishu
    }

    fn validate_channel(&self, channel: &RemoteImChannelConfig) -> Result<(), String> {
        if remote_im_credential_text(&channel.credentials, "appId").is_empty()
            || remote_im_credential_text(&channel.credentials, "appSecret").is_empty()
        {
            return Err(format!("feishu channel '{}' credentials invalid", channel.id));
        }
        Ok(())
    }

    fn send_outbound<'a>(
        &'a self,
        channel: &'a RemoteImChannelConfig,
        contact: &'a RemoteImContact,
        payload: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<String, RemoteImSdkSendError>> + Send + 'a>> {
        Box::pin(async move {
            let started = std::time::Instant::now();
            let items = remote_im_payload_content_items(payload);
            if items.is_empty() {
                return Err(RemoteImSdkSendError::definitely_not_sent(
                    "feishu outbound content is empty",
                ));
            }
            let token = self
                .tenant_access_token(channel)
                .await
                .map_err(RemoteImSdkSendError::definitely_not_sent)?;
            let receive_id_type = remote_im_credential_text(&channel.credentials, "receiveIdType");
            let receive_id_type = if receive_id_type.is_empty() {
                if remote_im_is_group_contact(contact) {
                    "chat_id".to_string()
                } else {
                    "open_id".to_string()
                }
            } else {
                receive_id_type
            };
            remote_im_log(
                "INFO",
                "feishu.send_outbound",
                serde_json::json!({
                    "channel_id": channel.id,
                    "contact_id": contact.id,
                    "remote_contact_id": contact.remote_contact_id,
                    "receive_id_type": receive_id_type,
                    "status": "开始",
                    "payload_summary": remote_im_payload_media_summary(payload)
                }),
            );
            let mut client_builder = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(12));
            #[cfg(target_os = "android")]
            {
                client_builder = features_system_commands::android_workspace_rootfs_installer::android_workspace_apply_static_webpki_roots(client_builder)
                    .map_err(RemoteImSdkSendError::definitely_not_sent)?;
            }
            let client = client_builder
                .build()
                .map_err(|err| {
                    let msg = format!("build feishu client failed: {err}");
                    remote_im_log(
                        "ERROR",
                        "feishu.send_outbound",
                        serde_json::json!({
                            "channel_id": channel.id,
                            "contact_id": contact.id,
                            "status": "失败",
                            "error": msg
                        }),
                    );
                    msg
                })
                .map_err(RemoteImSdkSendError::definitely_not_sent)?;
            let mut last_message_id = String::new();
            let mut delivered_any = false;
            for item in &items {
                let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
                let message_id = match item_type {
                    "text" => {
                        let text = item.get("text").and_then(Value::as_str).unwrap_or("").trim();
                        if text.is_empty() {
                            continue;
                        }
                        self.send_message(
                            &client,
                            &token,
                            &receive_id_type,
                            &contact.remote_contact_id,
                            "text",
                            serde_json::json!({ "text": text }),
                        )
                        .await
                        .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?
                    }
                    "image" => {
                        let raw = remote_im_content_item_bytes(item)
                            .await
                            .map_err(RemoteImSdkSendError::definitely_not_sent)
                            .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?;
                        let image_name = remote_im_content_item_name(item, "image.png");
                        let image_key = self
                            .upload_image_key(&client, &token, &image_name, raw)
                            .await
                            .map_err(RemoteImSdkSendError::definitely_not_sent)
                            .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?;
                        self.send_message(
                            &client,
                            &token,
                            &receive_id_type,
                            &contact.remote_contact_id,
                            "image",
                            serde_json::json!({ "image_key": image_key }),
                        )
                        .await
                        .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?
                    }
                    "file" => {
                        let raw = remote_im_content_item_bytes(item)
                            .await
                            .map_err(RemoteImSdkSendError::definitely_not_sent)
                            .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?;
                        let file_name = remote_im_content_item_name(item, "attachment.bin");
                        let file_mime = remote_im_content_item_mime(item, "application/octet-stream");
                        let file_key = self
                            .upload_file_key(&client, &token, &file_name, &file_mime, raw)
                            .await
                            .map_err(RemoteImSdkSendError::definitely_not_sent)
                            .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?;
                        self.send_message(
                            &client,
                            &token,
                            &receive_id_type,
                            &contact.remote_contact_id,
                            "file",
                            serde_json::json!({ "file_key": file_key }),
                        )
                        .await
                        .map_err(|err| err.after_confirmed_partial_delivery(delivered_any))?
                    }
                    _ => continue,
                };
                delivered_any = true;
                if !message_id.trim().is_empty() {
                    last_message_id = message_id;
                }
            }
            if last_message_id.trim().is_empty() {
                return Err(RemoteImSdkSendError::definitely_not_sent(
                    "feishu outbound content is empty",
                )
                .after_confirmed_partial_delivery(delivered_any));
            }
            remote_im_log(
                "INFO",
                "feishu.send_outbound",
                serde_json::json!({
                    "channel_id": channel.id,
                    "contact_id": contact.id,
                    "remote_contact_id": contact.remote_contact_id,
                    "receive_id_type": receive_id_type,
                    "status": "完成",
                    "message_id": last_message_id,
                    "duration_ms": started.elapsed().as_millis()
                }),
            );
            Ok(last_message_id)
        })
    }
}

