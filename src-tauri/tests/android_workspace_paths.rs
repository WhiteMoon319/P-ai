#[path = "../src/features/system/commands/android_workspace_paths.rs"]
mod android_workspace_paths;

use android_workspace_paths::*;

#[test]
fn runtime_base_should_live_next_to_llm_workspace() {
    let root = std::path::Path::new("app/llm-workspace");
    assert_eq!(
        android_workspace_runtime_base(root),
        std::path::Path::new("app")
            .join("runtime")
            .join("android-workspace")
            .join("default"),
    );
    assert_eq!(
        android_workspace_runtime_root(root),
        std::path::Path::new("app")
            .join("runtime")
            .join("android-workspace")
            .join("default")
            .join("linux"),
    );
}

#[test]
fn linux_runtime_gate_should_only_cover_exec_and_mcp() {
    assert!(android_workspace_tool_requires_linux_runtime("exec", false));
    assert!(android_workspace_tool_requires_linux_runtime(" read ", true));
    for tool_name in [
        "read",
        "read_file",
        "read_media",
        "write",
        "delete",
        "update",
        "move",
        "patch",
        "config",
        "reload",
        "operate",
    ] {
        assert!(
            !android_workspace_tool_requires_linux_runtime(tool_name, false),
            "file or app tool should not require Linux runtime: {tool_name}"
        );
    }
}

#[test]
fn guest_paths_should_map_to_host_workspace_paths() {
    let root = std::path::Path::new("/app/llm-workspace");
    assert_eq!(
        android_workspace_map_guest_path_to_host(
            root,
            std::path::Path::new("/workspace/skills/workspace-guide/SKILL.md"),
        ),
        root.join("skills").join("workspace-guide").join("SKILL.md"),
    );
    assert_eq!(
        android_workspace_map_guest_path_to_host(
            root,
            std::path::Path::new("/root/.pai/skills/workspace-guide/SKILL.md"),
        ),
        root.join("skills").join("workspace-guide").join("SKILL.md"),
    );
    assert_eq!(
        android_workspace_map_guest_path_to_host(
            root,
            std::path::Path::new("/root/.pai/private-organization/team.json"),
        ),
        root.join("private-organization").join("team.json"),
    );
    assert_eq!(
        android_workspace_map_guest_path_to_host(root, std::path::Path::new("/root/.pai/app_config.toml")),
        root.join("app_config.toml"),
    );
}

#[test]
fn tool_paths_should_allow_skills_and_project_dotfiles() {
    assert!(android_workspace_relative_path_is_tool_visible(
        std::path::Path::new("skills/workspace-guide/SKILL.md"),
        false,
    ));
    assert!(android_workspace_relative_path_is_tool_visible(
        std::path::Path::new(".gitignore"),
        false,
    ));
}

#[test]
fn tool_paths_should_block_internal_android_workspace_roots() {
    for path in [
        "runtime/linux/etc/os-release",
        "runtime/android-workspace/default/linux/etc/os-release",
        "tmp/proot/libproot.so",
        ".pai/internal.json",
        "mcp/server.json",
        "app_config.toml",
        "app_data.json",
    ] {
        assert!(
            !android_workspace_relative_path_is_tool_visible(std::path::Path::new(path), false),
            "internal path should be blocked: {path}"
        );
    }
}

#[test]
fn file_manager_paths_should_reject_escape_and_reserved_roots() {
    assert!(android_workspace_clean_relative_input("notes/today.md").is_ok());
    assert!(android_workspace_clean_relative_input("../outside.txt").is_err());
    assert!(android_workspace_clean_relative_input("notes/../../outside.txt").is_err());
    for path in [
        ".pai/plan/item.md",
        "skills/workspace-guide/SKILL.md",
        "mcp/server.json",
        "private-organization/team.json",
        "runtime/linux/etc/os-release",
        "runtime/android-workspace/default/linux/etc/os-release",
        "tmp/proot/libproot.so",
        "app_config.toml",
    ] {
        assert!(
            !android_workspace_relative_path_is_user_visible(std::path::Path::new(path), false),
            "file manager path should be hidden: {path}"
        );
    }
}

#[test]
fn glob_should_match_root_and_nested_files_with_double_star() {
    assert!(android_workspace_relative_matches_glob("note.txt", "**/*.txt").unwrap());
    assert!(android_workspace_relative_matches_glob("docs/note.txt", "**/*.txt").unwrap());
    assert!(android_workspace_relative_matches_glob("docs/deep/note.txt", "**/*.txt").unwrap());
    assert!(!android_workspace_relative_matches_glob("docs/note.md", "**/*.txt").unwrap());
    assert!(android_workspace_relative_matches_glob("note.txt", "*.txt").unwrap());
    assert!(!android_workspace_relative_matches_glob("docs/note.txt", "*.txt").unwrap());
}
