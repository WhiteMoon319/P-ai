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
#[path = "features/core/domain.rs"]
mod features_core_domain;
pub(crate) use features_core_domain::*;
// 时间语义已迁移至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::core::time_semantics::*;

// ==================== 配置与存储 ====================
#[path = "features/config/storage_and_stt.rs"]
mod features_config_storage_and_stt;
pub(crate) use features_config_storage_and_stt::*;
#[path = "features/config/app_data_layout.rs"]
mod features_config_app_data_layout;
pub(crate) use features_config_app_data_layout::*;
// app_layout_chat_dir / app_layout_chat_conversations_dir 已迁至
// crates/pai-backend message_store::paths（阶段 4）。
pub(crate) use pai_backend::message_store::paths::app_layout_chat_conversations_dir;
pub(crate) use pai_backend::message_store::paths::app_layout_chat_dir;
// crates/pai-backend message_store::meta（阶段 4）。
pub(crate) use pai_backend::message_store::meta::{
    build_conversation_preview_text, conversation_message_has_attachment,
    conversation_latest_summary_title, ConversationMetaPreviewMessage, ConversationMetaView,
};
// crates/pai-backend message_store::sqlite（阶段 4）。
pub(crate) use pai_backend::message_store::sqlite::ChatIndexConversationItem;
// crates/pai-backend tool_loop（阶段 4）。
pub(crate) use pai_backend::tool_loop::repeat_guard::ModelReply;
#[path = "features/chat/message_store/mod.rs"]
mod features_chat_message_store_mod;
pub(crate) use features_chat_message_store_mod::message_store;

// ==================== 独立图像生成 ====================
#[path = "features/image_generation.rs"]
mod features_image_generation;
pub(crate) use features_image_generation::*;

// ==================== 对话核心 ====================
#[path = "features/chat/message_semantics.rs"]
mod features_chat_message_semantics;
pub(crate) use features_chat_message_semantics::*;
#[path = "features/chat/conversation.rs"]
mod features_chat_conversation;
pub(crate) use features_chat_conversation::*;
#[path = "features/chat/message_attachment_projection.rs"]
mod features_chat_message_attachment_projection;
pub(crate) use features_chat_message_attachment_projection::*;
#[path = "features/chat/prompt_manager.rs"]
mod features_chat_prompt_manager;
pub(crate) use features_chat_prompt_manager::*;
#[path = "features/chat/conversation_prompt_service.rs"]
mod features_chat_conversation_prompt_service;
pub(crate) use features_chat_conversation_prompt_service::*;
#[path = "features/chat/conversation_service/mod.rs"]
mod features_chat_conversation_service_mod;
pub(crate) use features_chat_conversation_service_mod::*;
#[path = "features/chat/model_runtime.rs"]
mod features_chat_model_runtime;
pub(crate) use features_chat_model_runtime::*;
#[path = "features/chat/scheduler.rs"]
mod features_chat_scheduler;
pub(crate) use features_chat_scheduler::*;
#[path = "features/remote_im/channel_store.rs"]
mod features_remote_im_channel_store;
pub(crate) use features_remote_im_channel_store::*;
// markdown_filter 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::multilingual::markdown_filter::*;
#[path = "features/remote_im/onebot_v11_ws.rs"]
mod features_remote_im_onebot_v11_ws;
pub(crate) use features_remote_im_onebot_v11_ws::*;
#[cfg(target_os = "android")]
#[path = "features/remote_im/dingtalk_stream_android_stub.rs"]
mod features_remote_im_dingtalk_stream_android_stub;
pub(crate) use features_remote_im_dingtalk_stream_android_stub::*;
#[path = "features/remote_im/weixin_oc.rs"]
mod features_remote_im_weixin_oc;
pub(crate) use features_remote_im_weixin_oc::*;
#[path = "features/remote_im.rs"]
mod features_remote_im;
pub(crate) use features_remote_im::*;
#[path = "features/remote_im/maintenance.rs"]
mod features_remote_im_maintenance;
pub(crate) use features_remote_im_maintenance::*;
#[path = "features/remote_im_adapters.rs"]
mod features_remote_im_adapters;
pub(crate) use features_remote_im_adapters::*;

// ==================== 系统窗口与命令 ====================
#[path = "features/system/windowing.rs"]
mod features_system_windowing;
pub(crate) use features_system_windowing::*;
#[path = "features/system/sandbox.rs"]
mod features_system_sandbox;
pub(crate) use features_system_sandbox::*;
// local_port_service 已迁至 crates/pai-android-platform（阶段 5）。
pub(crate) use pai_android_platform::local_port_service::*;
#[path = "features/system/tools.rs"]
mod features_system_tools;
pub(crate) use features_system_tools::*;
#[cfg(target_os = "android")]
#[path = "features/system/updater_android_stub.rs"]
mod features_system_updater_android_stub;
pub(crate) use features_system_updater_android_stub::*;

// ==================== 记忆匹配 ====================
#[path = "features/memory/store.rs"]
mod features_memory_store;
pub(crate) use features_memory_store::*;
// matcher 已迁至 crates/pai-backend memory::matcher（阶段 4）。
pub(crate) use pai_backend::memory::matcher::*;
#[path = "features/memory/chat_history_search.rs"]
mod features_memory_chat_history_search;
pub(crate) use features_memory_chat_history_search::*;
#[path = "features/memory/providers.rs"]
mod features_memory_providers;
pub(crate) use features_memory_providers::*;

// ==================== MCP ====================
#[path = "features/mcp.rs"]
mod features_mcp;
pub(crate) use features_mcp::*;
#[path = "features/skill.rs"]
mod features_skill;
pub(crate) use features_skill::*;
#[path = "features/goal.rs"]
mod features_goal;
pub(crate) use features_goal::*;
#[path = "features/task.rs"]
mod features_task;
pub(crate) use features_task::*;
#[path = "features/delegate.rs"]
mod features_delegate;
pub(crate) use features_delegate::*;

#[path = "features/system/commands.rs"]
mod features_system_commands;
pub(crate) use features_system_commands::*;

#[cfg(target_os = "android")]
include!("native_bridge.rs");



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
    #[path = "features/tests.rs"]
mod features_tests;
pub(crate) use features_tests::*;
}
