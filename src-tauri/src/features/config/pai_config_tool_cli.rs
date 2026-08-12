// pai_config_tool.rs 纯逻辑已迁至 crates/pai-android-platform config（阶段 6）。
// 本文件仅保留桌面 CLI 入口 run_cli 及其依赖的路径探测
// （PAI_APP_ROOT / portable / ProjectDirs 标准配置目录）。

use std::path::PathBuf;

pub(crate) use pai_android_platform::config::pai_config_tool::*;

#[allow(dead_code)]
pub fn run_cli(args: &[String]) -> Result<String, String> {
    let (app_root, config_path, data_path, workspace_root) = detect_cli_paths()?;
    run_with_paths(app_root, config_path, data_path, workspace_root, args)
}

fn detect_cli_paths() -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), String> {
    if let Ok(root) = std::env::var("PAI_APP_ROOT") {
        let app_root = PathBuf::from(root);
        return Ok((
            app_root.clone(),
            app_root.join("app_config.toml"),
            app_root.join("app_data.json"),
            app_root.join("llm-workspace"),
        ));
    }

    if let Some(portable_root) = detect_portable_runtime_root() {
        let config_dir = portable_root.join("config");
        return Ok((
            portable_root.clone(),
            config_dir.join("app_config.toml"),
            config_dir.join("app_data.json"),
            portable_root.join("llm-workspace"),
        ));
    }

    let config_dir = resolve_standard_config_dir()?;
    let app_root = config_dir.clone();
    Ok((
        app_root.clone(),
        config_dir.join("app_config.toml"),
        config_dir.join("app_data.json"),
        app_root.join("llm-workspace"),
    ))
}

#[allow(dead_code)]
fn detect_portable_runtime_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let marker = exe_dir.join("PORTABLE");
    marker.exists().then(|| exe_dir.join("data"))
}

#[allow(dead_code)]
fn resolve_standard_config_dir() -> Result<PathBuf, String> {
    let dirs = directories::ProjectDirs::from("ai", "easycall", "p-ai")
        .ok_or_else(|| "无法定位标准配置目录".to_string())?;
    let path = dirs.config_dir().to_path_buf();
    std::fs::create_dir_all(&path)
        .map_err(|err| format!("创建标准配置目录失败 ({}): {err}", path.display()))?;
    Ok(path)
}
