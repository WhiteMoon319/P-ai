use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use futures_util::{future::AbortHandle, future::join_all, future::BoxFuture, StreamExt};
use image::ImageFormat;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rmcp::{schemars, ServiceExt};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use uuid::Uuid;

// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐
#[cfg(target_os = "android")]
use std::{
    ffi::OsString,
    fs::{self as std_fs, File as StdFile},
    io::{Read, Write},
    path::{Path as StdPath, PathBuf as StdPathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration as StdDuration, Instant},
};
#[cfg(target_os = "android")]
use walkdir::WalkDir;
#[cfg(target_os = "android")]
use zip::ZipArchive;

macro_rules! eprintln {
    ($($arg:tt)*) => {{
        runtime_log_info(format!($($arg)*));
    }};
}

fn bytes_to_lower_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

// ==================== 核心领域模型 ====================
include!("features/core/domain.rs");
include!("features/core/time_semantics.rs");

// ==================== 配置与存储 ====================
include!("features/config/storage_and_stt.rs");
include!("features/config/app_data_layout.rs");
include!("features/chat/message_store/mod.rs");

// ==================== 独立图像生成 ====================
include!("features/image_generation.rs");

// ==================== 对话核心 ====================
include!("features/chat/message_semantics.rs");
include!("features/chat/conversation.rs");
include!("features/chat/message_attachment_projection.rs");
include!("features/chat/prompt_manager.rs");
include!("features/chat/conversation_prompt_service.rs");
include!("features/chat/conversation_service/mod.rs");
include!("features/chat/model_runtime.rs");
include!("features/chat/scheduler.rs");
include!("features/remote_im/channel_store.rs");
include!("features/remote_im/markdown_filter.rs");
include!("features/remote_im/onebot_v11_ws.rs");
#[cfg(target_os = "android")]
include!("features/remote_im/dingtalk_stream_android_stub.rs");
include!("features/remote_im/weixin_oc.rs");
include!("features/remote_im.rs");
include!("features/remote_im/maintenance.rs");
include!("features/remote_im_adapters.rs");

// ==================== 系统窗口与命令 ====================
include!("features/system/windowing.rs");
include!("features/system/record_hotkey_probe.rs");
include!("features/system/windows_job.rs");
include!("features/system/sandbox.rs");
include!("features/system/local_port_service.rs");
include!("features/system/tools.rs");
#[cfg(target_os = "android")]
include!("features/system/updater_android_stub.rs");

// ==================== 记忆匹配 ====================
include!("features/memory/store.rs");
include!("features/memory/matcher.rs");
include!("features/memory/chat_history_search.rs");
include!("features/memory/providers.rs");

// ==================== MCP ====================
include!("features/mcp.rs");
include!("features/skill.rs");
include!("features/goal.rs");
include!("features/task.rs");
include!("features/delegate.rs");

include!("features/system/commands.rs");

#[cfg(target_os = "android")]
include!("native_bridge.rs");

fn should_enable_devtools() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    matches!(
        std::env::var("EASYCALL_DEVTOOLS")
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn unix_extend_process_path_from_login_shell() {
    use users::os::unix::UserExt;
    const PATH_BEGIN: &str = "__PAI_LOGIN_PATH_BEGIN__";
    const PATH_END: &str = "__PAI_LOGIN_PATH_END__";
    let shell = std::env::var_os("SHELL")
        .filter(|path| PathBuf::from(path).is_file())
        .or_else(|| {
            users::get_user_by_uid(users::get_current_uid())
                .map(|user| user.shell().as_os_str().to_os_string())
                .filter(|path| PathBuf::from(path).is_file())
        })
        .unwrap_or_else(|| "/bin/sh".into());
    let mut child = match std::process::Command::new(&shell)
        .args([
            "-ilc",
            "printf '__PAI_LOGIN_PATH_BEGIN__%s__PAI_LOGIN_PATH_END__' \"$PATH\"",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            runtime_log_warn(format!("[启动] 启动登录 shell 失败: err={err}"));
            return;
        }
    };
    let Some(stdout_pipe) = child.stdout.take() else {
        runtime_log_warn("[启动] 获取登录 shell stdout 失败，跳过环境同步".to_string());
        return;
    };
    // 读线程负责读 stdout 直到 EOF：shell 退出但 rc 启动的后台进程仍持有
    // stdout 写端时，阻塞只发生在子线程，不卡主线程（超时 kill 后由进程退出回收）。
    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = std::io::Read::read_to_end(&mut std::io::BufReader::new(stdout_pipe), &mut bytes);
        let _ = stdout_tx.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut stdout = None;
    let status = loop {
        if stdout.is_none() {
            if let Ok(buf) = stdout_rx.try_recv() {
                stdout = Some(buf);
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if stdout.is_some() => break status,
            Ok(_) => {}
            Err(err) => {
                runtime_log_warn(format!("[启动] 等待登录 shell 退出失败: err={err}"));
                return;
            }
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            runtime_log_warn("[启动] 读取登录 shell PATH 超时，跳过环境同步".to_string());
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let stdout = stdout.unwrap_or_default();
    if !status.success() {
        runtime_log_warn(format!(
            "[启动] 读取登录 shell PATH 失败: exit_code={}",
            status.code().unwrap_or(-1)
        ));
        return;
    }
    let Some(path_start) = stdout.rfind(PATH_BEGIN).map(|index| index + PATH_BEGIN.len()) else {
        runtime_log_warn(format!("[启动] 登录 shell 未返回 PATH 标记，跳过环境同步"));
        return;
    };
    let Some(path_end) = stdout[path_start..]
        .find(PATH_END)
        .map(|index| path_start + index)
    else {
        runtime_log_warn(format!("[启动] 登录 shell PATH 标记不完整，跳过环境同步"));
        return;
    };
    let login_path = std::ffi::OsString::from(stdout[path_start..path_end].trim());
    if login_path.is_empty() {
        runtime_log_warn(format!("[启动] 登录 shell PATH 为空，跳过环境同步"));
        return;
    }

    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let mut merged = Vec::<PathBuf>::new();
    for path in std::env::split_paths(&login_path).chain(std::env::split_paths(&current_path)) {
        if !path.as_os_str().is_empty() && !merged.iter().any(|item| item == &path) {
            merged.push(path);
        }
    }
    let Ok(path) = std::env::join_paths(merged) else {
        runtime_log_warn(format!("[启动] 合并登录 shell PATH 失败，跳过环境同步"));
        return;
    };
    std::env::set_var("PATH", path);
    runtime_log_info(format!("[启动] 已同步登录 shell PATH"));
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unix_extend_process_path_from_login_shell() {}



// Remote IM 命令包装





async fn refresh_conversation_meta_after_migration(state: AppState) {
    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(move || {
        let chat_index = collect_chat_index_items_from_storage(&state.data_path)?;
        let mut refreshed = 0usize;
        for item in &chat_index {
            if refresh_conversation_meta_shard_if_needed(&state.data_path, item.id.as_str())? {
                refreshed += 1;
            }
        }
        Ok::<(usize, usize), String>((chat_index.len(), refreshed))
    })
    .await;
    match result {
        Ok(Ok((total, refreshed))) => runtime_log_info(format!(
            "[启动] 完成，任务=迁移后会话meta预热，conversation_count={}，refreshed={}，elapsed_ms={}",
            total,
            refreshed,
            started.elapsed().as_millis()
        )),
        Ok(Err(err)) => runtime_log_warn(format!(
            "[启动] 失败，任务=迁移后会话meta预热，error={}，elapsed_ms={}",
            err,
            started.elapsed().as_millis()
        )),
        Err(err) => runtime_log_warn(format!(
            "[启动] 失败，任务=迁移后会话meta预热，error={}，elapsed_ms={}",
            err,
            started.elapsed().as_millis()
        )),
    }
}

// ==================== 桌面启动/关闭链路（Android 原生模式不走此段） ====================
// Android 由 native_bridge 的 JNI 初始化驱动，不经过 run_deferred_setup / 优雅退出 /
// 托盘 / 窗口等桌面启动逻辑；以下函数逐个 cfg 到非 Android，消除 Android 目标对
// NativeAppHandle / state / emit / tauri::async_runtime 的编译期引用。
/// 阶段 2 延迟初始化：在 backend_ready 之后异步执行，避免阻塞前端首屏渲染。



const APP_SHUTDOWN_STATE_IDLE: u8 = 0;
const APP_SHUTDOWN_STATE_RUNNING: u8 = 1;
const APP_SHUTDOWN_STATE_DONE: u8 = 2;
const BACKGROUND_SHUTDOWN_TIMEOUT_SECS: u64 = 60;

static APP_SHUTDOWN_STATE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(APP_SHUTDOWN_STATE_IDLE);








// Windows 通知品牌身份：进程级 AUMID + HKCU 注册 DisplayName/IconUri，
// 让未打包/裸 exe 场景下通知中心也按 PAI 品牌名+图标显示（上游 v0.57 迁入）。
#[cfg(target_os = "windows")]
fn windows_set_process_app_user_model_id() {
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        },
        UI::Shell::SetCurrentProcessExplicitAppUserModelID,
    };

    const AUMID: &str = "ai.easycall.app";
    const DISPLAY_NAME: &str = "PAI";

    // 1. 进程级 AUMID：让 toast 发送方身份与插件使用的 identifier 一致
    let aumid_wide: Vec<u16> = AUMID.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(aumid_wide.as_ptr()) };
    if result != 0 {
        runtime_log_warn(format!(
            "[通知] 设置进程 AppUserModelID 失败: 0x{:X}",
            result
        ));
    }

    // 2. HKCU 注册 AUMID（DisplayName/IconUri）：未打包应用的标准身份来源，
    //    无安装器快捷方式时通知中心也按品牌名+图标显示；幂等，每次启动重写
    let subkey: Vec<u16> = format!(r"Software\Classes\AppUserModelId\{AUMID}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut key_handle: HKEY = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key_handle,
            std::ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        runtime_log_warn(format!("[通知] 注册 AUMID 键失败: 0x{:X}", status));
        return;
    }
    let set_value = |name: &str, value: &str| {
        let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_wide: Vec<u16> = value.encode_utf16().collect();
        let status = unsafe {
            RegSetValueExW(
                key_handle,
                name_wide.as_ptr(),
                0,
                REG_SZ,
                value_wide.as_ptr() as *const u8,
                (value_wide.len() * 2) as u32,
            )
        };
        if status != ERROR_SUCCESS {
            runtime_log_warn(format!("[通知] 写入 AUMID {name} 失败: 0x{:X}", status));
        }
    };
    set_value("DisplayName", DISPLAY_NAME);
    if let Ok(exe) = std::env::current_exe() {
        set_value("IconUri", &exe.to_string_lossy());
    }
    unsafe {
        RegCloseKey(key_handle);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ========== Android 原生模式 ==========
    // 彻底拔掉 Tauri 运行时：Kotlin 不再继承 TauriActivity，后端由 PaiNative.init 通过
    // JNI 自建 Tokio runtime + AppState 驱动（见 native_bridge.rs）。此处仅兜底：若被
    // 误调用（如旧版 activity 生命周期残留），绝不允许再走 tauri::Builder 拉起 WebView。
    #[cfg(target_os = "android")]
    {
        init_backend_file_logging();
        install_backend_file_panic_hook();
        runtime_log_warn("[启动] Android 原生模式：tauri::Builder 已被移除，run() 不应被调用".to_string());
        // 阻塞挂起，避免 main 线程退出；真正的初始化由 JNI 入口完成。
        loop {
            std::thread::park();
        }
    }

    // ========== 桌面 Tauri 启动段 ==========
    // Android 原生模式不编译任何 tauri::Builder / generate_context 代码。
    // build.rs 已空壳化，tauri.conf.json 的 ACL metadata 不再生成，
    // 若 Android 编译期仍解析 generate_context!() 会报 UnknownManifest 错，故整段 cfg 隔离。
}

#[path = "features/config/pai_config_tool.rs"]
pub mod pai_config_tool;

#[cfg(test)]
mod tests {
    include!("features/tests.rs");
}
