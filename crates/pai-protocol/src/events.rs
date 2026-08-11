//! 事件清单（唯一来源：contracts/native-rpc/events.json）。
//!
//! Rust 推送到 native 事件队列、Kotlin pollEvents 消费。顺序由单一队列保证。

use std::collections::BTreeSet;

pub const CONTRACT_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../contracts/native-rpc/events.json"));

/// 全部已登记事件名。
pub fn registered_events() -> BTreeSet<String> {
    let value: serde_json::Value =
        serde_json::from_str(CONTRACT_JSON).expect("events.json 必须是合法 JSON");
    let mut out = BTreeSet::new();
    let events = value
        .get("events")
        .and_then(|v| v.as_object())
        .expect("events.json 缺少 events 对象");
    for name in events.keys() {
        assert!(!name.trim().is_empty(), "事件名不能为空");
        out.insert(name.clone());
    }
    out
}

pub fn is_registered(event: &str) -> bool {
    registered_events().contains(event)
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn contract_events_are_registered() {
        let events = registered_events();
        for required in [
            "app.keepAlive",
            "app.notification",
            "app.notification.clear",
            "chat.assistantDelta",
            "chat.roundFinished",
            "messageStore.migration.progress",
        ] {
            assert!(events.contains(required), "契约缺少事件: {required}");
        }
    }
}
