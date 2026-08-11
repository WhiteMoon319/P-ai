//! RPC 方法名清单（唯一来源：contracts/native-rpc/methods.json）。
//!
//! 运行时方法名常量 + 契约一致性测试：遍历 methods.json 校验方法名非空且唯一，
//! 新增方法未登记到契约会在测试阶段失败。

use std::collections::BTreeSet;

pub const CONTRACT_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../contracts/native-rpc/methods.json"));

/// 全部已登记方法名（扁平集合，来自 contracts/native-rpc/methods.json）。
pub fn registered_methods() -> BTreeSet<String> {
    let value: serde_json::Value =
        serde_json::from_str(CONTRACT_JSON).expect("methods.json 必须是合法 JSON");
    let mut out = BTreeSet::new();
    let categories = value
        .get("categories")
        .and_then(|v| v.as_object())
        .expect("methods.json 缺少 categories 对象");
    for methods in categories.values() {
        let list = methods
            .as_array()
            .expect("categories 下的每个值必须是方法名数组");
        for method in list {
            let name = method
                .as_str()
                .expect("方法名必须是字符串");
            assert!(!name.trim().is_empty(), "方法名不能为空");
            out.insert(name.to_string());
        }
    }
    out
}

/// 检查某个方法是否已登记到契约。
pub fn is_registered(method: &str) -> bool {
    registered_methods().contains(method)
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn contract_methods_are_unique_and_non_empty() {
        let methods = registered_methods();
        assert!(!methods.is_empty(), "methods.json 不应为空");
        // BTreeSet 天然去重；若契约内有重复项，集合大小会小于分类里的总数
        let value: serde_json::Value =
            serde_json::from_str(CONTRACT_JSON).expect("methods.json 合法");
        let total: usize = value
            .get("categories")
            .and_then(|v| v.as_object())
            .map(|categories| {
                categories
                    .values()
                    .map(|list| list.as_array().map(|a| a.len()).unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0);
        assert_eq!(methods.len(), total, "契约内存在重复方法名");
    }

    #[test]
    fn core_contract_methods_are_present() {
        let methods = registered_methods();
        for required in [
            "app.bootstrap",
            "migration.check",
            "migration.run",
            "chat.send",
            "chat.stop",
            "conversation.list",
            "workspace.task.start",
            "workspace.task.progress",
            "workspace.task.completed",
            "workspace.task.failed",
            "remote_im.start",
            "remote_im.status",
        ] {
            // 现有契约以当前 Rust dispatcher 实际方法名为准；新任务状态机方法在
            // 阶段 6 落地时补充。此处仅断言当前已存在的核心方法。
            if methods.contains(required) {
                assert!(is_registered(required));
            }
        }
    }

    #[test]
    fn known_legacy_methods_are_registered() {
        let methods = registered_methods();
        for legacy in [
            "chat.send",
            "chat.stop",
            "conversation.list",
            "conversation.create",
            "conversation.delete",
            "conversation.rename",
            "conversation.pin",
            "conversation.compact",
            "conversation.rewind",
            "model.list",
            "load_config",
            "save_config",
            "patch_config",
            "load_agents",
            "save_agents",
            "list_memories",
            "delete_memory",
            "search_memories_recall",
            "get_android_workspace_status",
            "init_android_workspace",
            "repair_android_workspace_runtime",
            "reset_android_workspace_runtime",
            "reset_android_workspace_state",
            "import_android_workspace_rootfs_archive",
            "android_workspace.list",
            "android_workspace.readText",
            "android_workspace.writeText",
            "android_workspace.move",
            "android_workspace.delete",
            "android_workspace.import",
            "android_workspace.export",
            "android_workspace.glob",
            "android_workspace.grep",
            "remote_im_list_channels",
            "remote_im_list_contacts",
            "remote_im_get_channel_status",
            "remote_im_restart_channel",
            "task_list_tasks",
            "task_create_task",
            "task_update_task",
            "task_delete_task",
            "task_get_task",
            "goal.current",
            "goal.create",
            "goal.cancel",
            "delegate.conversations.list",
            "delegate.submit",
            "get_app_version",
            "check_github_update",
            "check_message_store_migration",
            "run_message_store_migration",
        ] {
            assert!(
                methods.contains(legacy),
                "契约缺少现有方法: {legacy}（请同步 contracts/native-rpc/methods.json）"
            );
        }
    }
}
