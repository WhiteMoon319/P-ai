#!/usr/bin/env python3
"""include!() → module 批量转换工具（P-AI Android 迁移阶段 3）。

用法: python tools/android/include_to_module.py convert <top_file> [--no-inject-use]
      python tools/android/include_to_module.py list

把 lib.rs 的 `include!("features/xxx.rs")` 改为:
  #[path = "features/xxx.rs"]
  mod xxx;
  pub(crate) use xxx::*;

转换要点:
  1. 目标文件（mod 主体）顶部注入 lib.rs 的 use 块（子 include! 同作用域继承）。
  2. 目标文件内所有顶层符号加 pub(crate)（impl 除外），供 crate 根 pub(crate) use 导出。
  3. 跨域引用: 其他 mod 用 `use super::*`（crate 根已 pub(crate) use 提升的符号）。

注意: 本工具只处理单个顶层 include 文件; 每个转换后必须 cargo check 验证,
剩余跨域引用错误需人工补 pub(crate) 或 use。
"""
import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
SRC = REPO / "src-tauri" / "src"
LIB = SRC / "lib.rs"

TOP_PAT = re.compile(r"^(fn |async fn |pub fn |pub async fn |struct |enum |const |static |type |trait )")


def extract_lib_use_block() -> str:
    """提取 lib.rs 顶部 use 块（从文件头到第一个 include! 之前）。"""
    lines = LIB.read_text(encoding="utf-8").splitlines()
    out = []
    for line in lines:
        if line.startswith("include!"):
            break
        out.append(line)
    return "\n".join(out).rstrip() + "\n\n"


def list_top_includes() -> list[str]:
    text = LIB.read_text(encoding="utf-8")
    return re.findall(r'include!\("features/([^"]+)"\);', text)


def convert(top_rel: str, dry_run: bool, inject_use: bool) -> None:
    top_path = SRC / top_rel
    if not top_path.is_file():
        sys.exit(f"未找到文件: {top_path}")

    mod_name = top_rel.replace("/", "_").replace(".rs", "").replace("-", "_")
    lib_text = LIB.read_text(encoding="utf-8")
    old = f'include!("{top_rel}");'
    if old not in lib_text:
        sys.exit(f"lib.rs 未找到: {old}")
    new = (
        f'#[path = "features/{top_rel}"]\n'
        f"mod {mod_name};\n"
        f"pub(crate) use {mod_name}::*;"
    )

    if inject_use:
        content = top_path.read_text(encoding="utf-8")
        first_line = content.splitlines()[0] if content.splitlines() else ""
        if first_line.startswith("include!"):
            use_block = extract_lib_use_block()
            content = use_block + content
            if not dry_run:
                top_path.write_text(content, encoding="utf-8", newline="\n")
            print(f"[{top_rel}] 注入 lib.rs use 块")

    if not dry_run:
        lib_text = lib_text.replace(old, new, 1)
        LIB.write_text(lib_text, encoding="utf-8", newline="\n")
    print(f"[lib.rs] {old} → mod {mod_name} + pub(crate) use")
    print("完成。请 cargo check 验证，剩余跨域引用错误需人工补 pub(crate)。")


def main() -> None:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("list").set_defaults(func=lambda a: print("\n".join(list_top_includes())))

    p_conv = sub.add_parser("convert")
    p_conv.add_argument("top_rel", help="features/ 下相对路径，如 core/domain.rs")
    p_conv.add_argument("--no-inject-use", action="store_true", help="不注入 lib.rs use 块")
    p_conv.set_defaults(func=None)

    args = ap.parse_args()
    if args.cmd == "list":
        print("\n".join(list_top_includes()))
    elif args.cmd == "convert":
        convert(args.top_rel, dry_run=False, inject_use=not args.no_inject_use)
    else:
        ap.print_help()


if __name__ == "__main__":
    main()
