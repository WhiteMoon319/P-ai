use jni::objects::{JClass, JString};
use jni::sys::{jstring};
use jni::JNIEnv;

/// 提取 Android 应用数据目录（Kotlin 传入的 filesDir 等）。
unsafe fn app_root_from_env(env: &mut JNIEnv, input: JString) -> Result<std::path::PathBuf, String> {
    let raw: String = env
        .get_string(&input)
        .map_err(|err| format!("读取 appRoot 失败: {err}"))?
        .into();
    let root = std::path::PathBuf::from(raw);
    if root.as_os_str().is_empty() {
        return Err("appRoot 为空".to_string());
    }
    Ok(root)
}

// ========== JNI 导出 ==========

/// Java_com_whitemoon319_pai_native_PaiNative_init(String appRoot) -> String
/// 返回 "ok" 或错误信息（便于 Kotlin 侧直接弹错）。
#[no_mangle]
pub extern "system" fn Java_com_whitemoon319_pai_native_PaiNative_init(
    mut env: JNIEnv,
    _class: JClass,
    app_root: JString,
) -> jstring {
    let result = unsafe { app_root_from_env(&mut env, app_root) }
        .and_then(init_native_runtime)
        .map(|()| "ok".to_string())
        .unwrap_or_else(|err| format!("nativeInit 失败: {err}"));
    match env.new_string(result) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Java_com_whitemoon319_pai_native_PaiNative_call(String requestJson) -> String
/// 同步执行 JSON-RPC 并返回响应 JSON；失败时返回 JSON-RPC error 结构。
#[no_mangle]
pub extern "system" fn Java_com_whitemoon319_pai_native_PaiNative_call(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    let response = native_call_inner(&mut env, &request_json);
    match env.new_string(response) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Java_com_whitemoon319_pai_native_PaiNative_pollEvents() -> String
/// 拉取并清空待下发事件（流式 delta / 工具事件等），返回 JSON 数组。
#[no_mangle]
pub extern "system" fn Java_com_whitemoon319_pai_native_PaiNative_pollEvents(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let events = drain_native_delta_events();
    match env.new_string(events) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn native_call_inner(env: &mut JNIEnv, request_json: &JString) -> String {
    let request_text: String = match env.get_string(request_json) {
        Ok(s) => s.into(),
        Err(err) => {
            return ide_chat_jsonrpc_error(
                None,
                -32600,
                format!("读取请求失败: {err}"),
            )
            .to_string();
        }
    };

    let request: Value = match serde_json::from_str(&request_text) {
        Ok(value) => value,
        Err(err) => {
            return ide_chat_jsonrpc_error(None, -32700, format!("invalid json: {err}")).to_string();
        }
    };
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    if method.trim().is_empty() {
        return ide_chat_jsonrpc_error(id, -32600, "missing method").to_string();
    }

    let runtime = match native_runtime() {
        Ok(runtime) => runtime.clone(),
        Err(err) => {
            return ide_chat_jsonrpc_error(id, -32000, err).to_string();
        }
    };

    // 同步边界：使用 NativeDispatcherImpl 把 dispatch 提交到原生 Tokio worker 执行。
    let dispatcher = NativeDispatcherImpl(runtime.clone());
    let result = dispatcher.dispatch(&method, params, id.clone());
    match result {
        Ok(value) => ide_chat_jsonrpc_success(id, value).to_string(),
        Err(err) => ide_chat_jsonrpc_error(id, -32000, err).to_string(),
    }
}