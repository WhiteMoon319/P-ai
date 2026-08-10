#!/usr/bin/env python3
"""批量把 android_workspace.rs 的文件管理 tauri::command 改造成「薄壳 + *_ws_inner」模式。

规则：对指定命令，把
  #[tauri::command]
  fn <name>(state: State<'_, AppState>, ...) -> Result<T, String> {
    #[cfg(target_os = "android")] { ... } #[cfg(not...)] { ... }
  }
拆成：
  #[tauri::command]
  fn <name>(state: State<'_, AppState>, ...) -> Result<T, String> {
    <name>_ws_inner(state.inner(), ...)
  }
  fn <name>_ws_inner(state: &AppState, ...) -> Result<T, String> { 原函数体，&state -> state }

只处理参数列表为 (State, String/Option<String>/bool/Option<bool>) 的纯文本命令。
"""

import re
import sys

PATH = sys.argv[1]
CMDS = [
    "read_android_workspace_text",
    "write_android_workspace_text",
    "move_android_workspace_file",
    "glob_android_workspace_files",
    "grep_android_workspace_files",
    "delete_file_from_android_workspace",
    "import_file_to_android_workspace",
    "export_file_from_android_workspace",
]

with open(PATH, "r", encoding="utf-8") as f:
    src = f.read()

for cmd in CMDS:
    # 定位 #[tauri::command] fn cmd( ... ) -> Result<...> {
    pat = re.compile(
        r"(#\[tauri::command\]\s*\n"
        r"fn " + re.escape(cmd) + r"\s*\("
        r"(.*?)\)\s*->\s*Result<([^,>]+),\s*String>\s*\{\n)"
        r"(.*?)\n\}\n\n",
        re.DOTALL,
    )
    m = pat.search(src)
    if not m:
        print(f"!! 未匹配 {cmd}")
        continue
    params = m.group(2)  # state: State<'_, AppState>, path: String, ...
    ret_type = m.group(3)
    body = m.group(4)

    # 解析参数：state 参数单独处理（State<'_, AppState> 内含逗号，需合并前两段）
    raw_parts = [p.strip() for p in params.split(",") if p.strip()]
    if not raw_parts or "State<'_" not in raw_parts[0]:
        print(f"!! {cmd} state 参数格式不符: {params.strip()[:60]}")
        continue
    # raw_parts[0] = "state: State<'_"，raw_parts[1] = "AppState>"（合并为 state 参数）
    # 其余为真实参数
    if len(raw_parts) > 1 and raw_parts[1].endswith(">"):
        rest_raw = raw_parts[2:]
    else:
        rest_raw = raw_parts[1:]
    param_parts = rest_raw
    rest_params = ", ".join(param_parts)
    rest_names = [p.split(":")[0].strip() for p in param_parts]

    # inner 函数体：&state -> state
    inner_body = body.replace("&state", "state")
    # 非 android 分支的 let _ = state 保留（参数名一致）

    # 薄壳：调用 inner
    args = ["state.inner()"] + rest_names
    shell = (
        f"#[tauri::command]\n"
        f"fn {cmd}(\n"
        f"    state: State<'_, AppState>,\n"
        + (",\n".join(f"    {p}" for p in param_parts) + ",\n" if rest_params else "")
        + f") -> Result<{ret_type}, String> {{\n"
        f"    {cmd}_ws_inner({', '.join(args)})\n"
        f"}}\n\n"
    )
    inner = (
        f"/// ws 端调用版。\n"
        f"fn {cmd}_ws_inner(\n"
        f"    state: &AppState,\n"
        + (",\n".join(f"    {p}" for p in param_parts) + ",\n" if rest_params else "")
        + f") -> Result<{ret_type}, String> {{\n"
        + inner_body
        + "\n}\n\n"
    )
    src = src[: m.start()] + shell + inner + src[m.end():]
    print(f"✓ 已转换 {cmd}")

with open(PATH, "w", encoding="utf-8") as f:
    f.write(src)
print("完成")
