#!/usr/bin/env python3
"""include!() → module 批量转换工具（P-AI Android 迁移阶段 3）。

用法: python tools/android/include_to_module.py <domain> [--dry-run]

把 src-tauri/src/features/<domain>.rs 转换为:
  - features/<domain>/mod.rs（保留子 include! 与必要的 use super::*）
  - 顶层符号批量加 pub(crate)（impl 除外）
  - lib.rs 对应 include! 改为 mod + pub(crate) use

注意事项:
  - 只处理单个顶层域文件，其内部子 include! 保持原样（同 mod 作用域互见）。
  - 跨域符号经 lib.rs 的 pub(crate) use 提升到 crate 根，目标 mod 用 use super::* 引入。
  - 转换后必须 cargo check 验证；本工具不保证一次通过，剩余编译错误需人工修。
"""
import argparse
import re
import sys
from pathlib import Path

SRC = Path("src-tauri/src/features")
LIB = Path("src-tauri/src/lib.rs")

TOP_PAT = re.compile(
    r"^(fn |async fn |pub fn |pub async fn |struct |enum |const |static |type |trait )"
)
IMPL_PAT = re.compile(r"^impl ")


def convert_domain(domain: str, dry_run: bool) -> None:
    top_file = SRC / f"{domain}.rs"
    if not top_file.exists():
        sys.exit(f"未找到域文件: {top_file}")
    domain_dir = SRC / domain
    if not domain_dir.is_dir():
        sys.exit(f"未找到域目录（需要先 mkdir）: {domain_dir}")

    # 1) 读取旧文件内容，构造 mod.rs
    content = top_file.read_text(encoding="utf-8")
    # 子 include 保留；去掉可能的 mod.rs 自身 include
    mod_content = content

    # 2) 顶层符号加 pub(crate)（仅子文件，不含 mod.rs 自身）
    changed_files = []
    for child in sorted(domain_dir.glob("*.rs")):
        if child.name == "mod.rs":
            continue
        lines = child.read_text(encoding="utf-8").splitlines(keepends=True)
        out = []
        modified = False
        for line in lines:
            stripped = line.lstrip()
            indent = line[: len(line) - len(stripped)]
            if TOP_PAT.match(stripped) and not stripped.startswith("pub "):
                out.append(f"{indent}pub(crate) {stripped}")
                modified = True
            else:
                out.append(line)
        if modified:
            if not dry_run:
                child.write_text("".join(out), encoding="utf-8", newline="\n")
            changed_files.append(str(child))

    # 3) lib.rs: include! → mod + pub(crate) use
    lib_text = LIB.read_text(encoding="utf-8")
    old = f'include!("features/{domain}.rs");'
    new = (
        f'#[path = "features/{domain}/mod.rs"]\n'
        f"mod {domain.replace('/', '_')};\n"
        f"pub(crate) use {domain.replace('/', '_')}::*;"
    )
    if old in lib_text:
        lib_text = lib_text.replace(old, new, 1)
        if not dry_run:
            LIB.write_text(lib_text, encoding="utf-8", newline="\n")
        print(f"[lib.rs] {old} → mod + pub(crate) use")
    else:
        print(f"[警告] lib.rs 未找到 {old}")

    print(f"[{domain}] 顶层符号 pub(crate) 化文件 {len(changed_files)} 个:")
    for f in changed_files:
        print(f"  + {f}")
    print("完成。请运行 cargo check 验证，剩余跨域引用错误需人工修复。")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("domain", help="域文件名（不含 .rs），如 core/domain")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    convert_domain(args.domain, args.dry_run)


if __name__ == "__main__":
    main()
