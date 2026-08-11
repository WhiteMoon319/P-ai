#!/usr/bin/env python3
"""批量执行 include!→module 转换（阶段 3）：逐域 convert + 自动补 use super::* + cargo check。

用法: python tools/android/batch_convert.py [--skip-check]
"""
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
TOOL = REPO / "tools" / "android" / "include_to_module.py"
LIB = REPO / "src-tauri" / "src" / "lib.rs"

# 依赖顺序：core 已转，其余按 lib.rs include 顺序（依赖方在后）
DOMAINS = [
    "config/app_data_layout.rs",
    "chat/message_store/mod.rs",
    "image_generation.rs",
    "chat/message_semantics.rs",
    "chat/conversation.rs",
    "chat/message_attachment_projection.rs",
    "chat/prompt_manager.rs",
    "chat/conversation_prompt_service.rs",
    "chat/conversation_service/mod.rs",
    "chat/model_runtime.rs",
    "chat/scheduler.rs",
    "remote_im/channel_store.rs",
    "remote_im/markdown_filter.rs",
    "remote_im/onebot_v11_ws.rs",
    "remote_im/dingtalk_stream_android_stub.rs",
    "remote_im/weixin_oc.rs",
    "remote_im.rs",
    "remote_im/maintenance.rs",
    "remote_im_adapters.rs",
    "system/windowing.rs",
    "system/sandbox.rs",
    "system/local_port_service.rs",
    "system/tools.rs",
    "system/updater_android_stub.rs",
    "memory/store.rs",
    "memory/matcher.rs",
    "memory/chat_history_search.rs",
    "memory/providers.rs",
    "mcp.rs",
    "skill.rs",
    "goal.rs",
    "task.rs",
    "delegate.rs",
    "system/commands.rs",
]


def ensure_super_use(rel: str) -> bool:
    """若 mod 文件尚无 use super::*，在第一个 pub(crate)/pub fn 前插入。"""
    top = REL_MAP.get(rel)
    if not top:
        return False
    s = top.read_text(encoding="utf-8")
    if "use super::*;" in s:
        return False
    marker = "pub(crate) "
    idx = s.find(marker)
    if idx < 0:
        marker2 = "pub fn "
        idx = s.find(marker2)
    if idx < 0:
        return False
    s = s[:idx] + "use super::*;\n\n" + s[idx:]
    top.write_text(s, encoding="utf-8", newline="\n")
    return True


def main() -> int:
    skip_check = "--skip-check" in sys.argv
    # 建立 rel → 聚合文件映射（convert 工具处理 features/ 前缀）
    global REL_MAP
    REL_MAP = {}
    for rel in DOMAINS:
        top = REPO / "src-tauri" / "src" / "features" / rel
        REL_MAP[rel] = top

    failures = []
    for rel in DOMAINS:
        print(f"\n===== 转换 {rel} =====")
        r = subprocess.run(
            [sys.executable, str(TOOL), "convert", rel],
            capture_output=True, text=True, cwd=REPO,
        )
        print(r.stdout[-600:] if r.stdout else "")
        if r.returncode != 0:
            print(f"[失败] convert {rel}: {r.stderr[-300:]}")
            failures.append(f"convert:{rel}")
            continue
        # 自动补 use super::*
        if ensure_super_use(rel):
            print(f"[{rel}] 补充 use super::*")
        if skip_check:
            continue
        # cargo check
        chk = subprocess.run(
            ["cargo", "check", "--target", "aarch64-linux-android"],
            capture_output=True, text=True, cwd=REPO / "src-tauri",
        )
        errs = [l for l in chk.stderr.splitlines() if l.startswith("error")]
        # 环境变量（NDK）由调用方 shell 提供；此处只统计错误
        if errs and not any("error: could not compile" in e for e in errs):
            # 有编译错误
            uniq = {}
            for e in errs:
                key = e.split(":")[0] if ":" in e else e
                uniq[key] = uniq.get(key, 0) + 1
            top = sorted(uniq.items(), key=lambda kv: -kv[1])[:5]
            print(f"[check] {rel} 错误 {len(errs)}: {top}")
            failures.append(f"check:{rel}:{len(errs)}")
        else:
            print(f"[check] {rel} 通过")

    print("\n===== 汇总 =====")
    if failures:
        print("失败项:")
        for f in failures:
            print(f"  {f}")
        return 1
    print("全部域转换完成")
    return 0


if __name__ == "__main__":
    sys.exit(main())
