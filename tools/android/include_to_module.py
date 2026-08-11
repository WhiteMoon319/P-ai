#!/usr/bin/env python3
"""include!() → module 批量转换工具（P-AI Android 迁移阶段 3）。

用法:
  python tools/android/include_to_module.py list
  python tools/android/include_to_module.py convert <top_file> [--no-inject-use] [--no-inject-pub]

把 lib.rs 的 `include!("features/xxx.rs")` 改为:
  #[path = "features/xxx.rs"]
  mod xxx;
  pub(crate) use xxx::*;

转换要点:
  1. 目标文件（mod 主体）顶部注入 lib.rs 的 use 块（子 include! 同作用域继承）。
  2. 目标文件及其递归子 include 文件内所有顶层符号加 pub(crate)（impl 除外）。
  3. struct 字段 / enum 变体字段加 pub(crate)（原 include! 单入口下字段虽未标 pub 但
     同 crate 根作用域可访问；mod 隔离后必须 pub(crate)）。
  4. 跨域引用: 其他 mod 用 `use super::*`（crate 根已 pub(crate) use 提升的符号）。

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

TOP_PAT = re.compile(
    r"^(fn |async fn |pub fn |pub async fn |struct |enum |const |static |type |trait )"
)
IMPL_PAT = re.compile(r"^(impl |pub impl |unsafe impl )")


def extract_lib_use_block() -> str:
    """提取 lib.rs 顶部的 use 与 cfg 属性行（供子 mod 注入）。

    逐行收集，支持多行 use（`use X::{ ... };`）：
      - use 行（含续行：以缩进开头的非语句行）收集
      - #[cfg] 属性、空行、// 注释收集
      - 遇到 include!/mod/函数定义等代码即停止
    多行 use 的续行判定：当前在 use 块内（brace 未闭合）时收集所有行。
    """
    lines = LIB.read_text(encoding="utf-8").splitlines()
    out = []
    brace_depth = 0
    skip_cfg_block = False
    for line in lines:
        stripped = line.strip()
        if brace_depth > 0:
            # 多行 use 续行：持续收集直到 brace 闭合（cfg 门控块整体跳过）
            if not skip_cfg_block:
                out.append(line)
            brace_depth += stripped.count("{") - stripped.count("}")
            if brace_depth <= 0:
                skip_cfg_block = False
            continue
        if (
            stripped.startswith("include!(")
            or stripped.startswith("mod ")
            or stripped.startswith("pub(crate) use ")
        ):
            break
        if stripped.startswith("use "):
            if skip_cfg_block:
                # cfg 门控的 use 块（lib.rs android 段）不注入，避免两段 use std 重名
                brace_depth = stripped.count("{") - stripped.count("}")
                continue
            out.append(line)
            brace_depth = stripped.count("{") - stripped.count("}")
            continue
        if stripped.startswith("#[cfg"):
            skip_cfg_block = True
            continue
        if stripped == "" or stripped.startswith("//"):
            out.append(line)
            continue
        break
    return "\n".join(out).rstrip() + "\n\n"


def list_top_includes() -> list[str]:
    text = LIB.read_text(encoding="utf-8")
    return re.findall(r'include!\("features/([^"]+)"\);', text)


def collect_include_files(root: Path) -> list[Path]:
    """递归收集 root 文件及其 include!() 的子文件（相对 root 所在目录）。"""
    seen: set[Path] = set()
    stack = [root]
    while stack:
        cur = stack.pop()
        if cur in seen:
            continue
        seen.add(cur)
        text = cur.read_text(encoding="utf-8")
        base = cur.parent
        for m in re.finditer(r'include!\("([^"]+)"\);', text):
            child = (base / m.group(1)).resolve()
            if child.suffix == ".rs" and child.is_file() and child not in seen:
                stack.append(child)
    return sorted(seen, key=lambda p: str(p))


def inject_pub_crate(path: Path) -> int:
    """给文件顶层符号加 pub(crate)（impl 除外；仅无缩进行），返回修改行数。"""
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out = []
    changed = 0
    for line in lines:
        stripped = line.lstrip()
        indent = line[: len(line) - len(stripped)]
        # 仅处理无缩进的顶层符号（impl 内方法有缩进，不应注入）
        if (
            indent == ""
            and TOP_PAT.match(stripped)
            and not stripped.startswith("pub ")
            and not IMPL_PAT.match(stripped)
        ):
            out.append(f"{indent}pub(crate) {stripped}")
            changed += 1
        else:
            out.append(line)
    if changed:
        path.write_text("".join(out), encoding="utf-8", newline="\n")
    return changed


def inject_pub_crate_fields(path: Path) -> int:
    """给 struct 字段与 enum 变体字段加 pub(crate)，返回修改行数。

    算法：逐行扫描，维护花括号深度。当某行以 struct/enum 定义开头（深度 0 且非 impl）
    时进入「定义体」模式；在定义体内（深度 > 0）识别字段行：
      - 命名段: `ident: Type`（排除 pub 开头、属性、泛型方法形态）
      - 元组段: `Type,` / `Type)`（enum 变体数据、tuple struct）
    深度回到 0 时退出定义体。
    """
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out = []
    changed = 0
    depth = 0
    in_body = False
    in_struct = False  # True=struct 体，False=enum 体（enum 变体字段不允许 pub 修饰）
    in_attr = False  # 属性块（#[serde(...)] 多行）内不注入
    prev_field_name_line = False  # 上一行是「字段名:」结尾（类型续行，跳过注入）
    for line in lines:
        stripped = line.lstrip()
        indent = line[: len(line) - len(stripped)]
        if prev_field_name_line:
            # 字段类型续行（上一行是「字段名:」）：不注入，仅复位标志
            prev_field_name_line = False
            out.append(line)
            continue
        if in_attr:
            # 属性块续行：直到 ] 闭合
            out.append(line)
            if "]" in stripped:
                in_attr = False
            continue
        if not in_body and depth == 0:
            # 顶层 struct/enum 定义（含 pub(crate) 前缀与 where 子句前）
            m = re.match(
                r"^(pub\(crate\) )?(struct|enum)\s+[A-Za-z0-9_]+", stripped
            )
            if m and not IMPL_PAT.match(stripped):
                in_body = True
                in_struct = m.group(2) == "struct"
                depth += stripped.count("{") - stripped.count("}")
                out.append(line)
                continue
            out.append(line)
            continue
        if in_body:
            depth += stripped.count("{") - stripped.count("}")
            if stripped.startswith("#"):
                # 属性行（#[serde(...)] 可能多行）：进入属性块跳过注入
                out.append(line)
                if "]" not in stripped:
                    in_attr = True
                if depth <= 0:
                    in_body = False
                continue
            if stripped.strip() == "":
                out.append(line)
                if depth <= 0:
                    in_body = False
                continue
            if stripped.startswith("}"):
                out.append(line)
                if depth <= 0:
                    in_body = False
                continue
            if stripped.startswith("pub ") or stripped.startswith("pub(crate) "):
                out.append(line)
                if depth <= 0:
                    in_body = False
                continue
            if stripped.startswith("//"):
                out.append(line)
                continue
            # 命名段字段: `ident: Type`（字段名后冒号；排除 where 泛型/方法签名）
            if in_struct and re.match(r"^[A-Za-z_][A-Za-z0-9_]*\s*:", stripped):
                out.append(f"{indent}pub(crate) {stripped}")
                changed += 1
                # 字段名: 后无类型（类型在下一行）→ 标记续行
                if re.match(r"^[A-Za-z_][A-Za-z0-9_]*\s*:\s*$", stripped):
                    prev_field_name_line = True
                if depth <= 0:
                    in_body = False
                continue
            # 元组段字段: `Type,` 或 `Type)`（enum 变体数据 / tuple struct）。
            # 排除纯变体名（`C,` / `A,`）与小写开头的路径续行（`std::...`，
            # 如 `by_provider_model:` 换行后的类型续行）——元组字段类型通常大写开头。
            if (
                in_struct
                and re.match(r"^[A-Za-z][A-Za-z0-9_:<>\[\]\(\)]*\s*[,)]\s*$", stripped)
                and not re.match(r"^[A-Z][A-Za-z0-9_]*\s*[,)]\s*$", stripped)
                and not re.match(r"^[a-z][A-Za-z0-9_:<>\[\]\(\)]*::", stripped)
            ):
                out.append(f"{indent}pub(crate) {stripped}")
                changed += 1
                if depth <= 0:
                    in_body = False
                continue
            out.append(line)
            if depth <= 0:
                in_body = False
            continue
        out.append(line)
    if changed:
        path.write_text("".join(out), encoding="utf-8", newline="\n")
    return changed


def inject_pub_crate_methods(path: Path) -> int:
    """给 inherent impl 块内的方法加 pub(crate)，返回修改行数。

    只处理 `impl X { ... }`（inherent impl，无 `for`）；trait impl
    （`impl Trait for X`）方法不允许可见性修饰，跳过。方法 = impl 体内
    顶层 fn（缩进 4 空格，depth 在 impl 花括号内）。
    """
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out = []
    changed = 0
    depth = 0
    in_impl = False
    is_trait_impl = False
    pending_impl = None  # 缓冲 impl 头行，等待判断是否为 trait impl
    for line in lines:
        stripped = line.lstrip()
        indent = line[: len(line) - len(stripped)]
        if not in_impl and pending_impl is None:
            m = re.match(r"^(pub\(crate\) )?impl\s+([^{]+?)(?:\s*\{)?\s*$", stripped)
            if m and not stripped.startswith("#"):
                body = m.group(2)
                # 判断是否 trait impl：`impl Trait for Type`
                is_trait = " for " in body
                pending_impl = line
                in_impl = True
                is_trait_impl = is_trait
                depth = stripped.count("{") - stripped.count("}")
                if depth <= 0:
                    # impl X where ... { 多行头，等待续行
                    pass
                out.append(line)
                continue
            out.append(line)
            continue
        if in_impl:
            depth += stripped.count("{") - stripped.count("}")
            # 方法：impl 体直接子级 fn（缩进 4，非 pub 开头，非 trait impl）
            if (
                not is_trait_impl
                and re.match(r"^fn |^async fn |^pub fn |^pub async fn ", stripped)
                and not stripped.startswith("pub ")
                and indent == "    "
            ):
                out.append(f"{indent}pub(crate) {stripped}")
                changed += 1
                if depth <= 0:
                    in_impl = False
                    pending_impl = None
                continue
            if stripped.startswith("}") and depth <= 0:
                in_impl = False
                pending_impl = None
                out.append(line)
                continue
            out.append(line)
            if depth <= 0:
                in_impl = False
                pending_impl = None
            continue
        out.append(line)
    if changed:
        path.write_text("".join(out), encoding="utf-8", newline="\n")
    return changed


def convert(top_rel: str, dry_run: bool, inject_use: bool, inject_pub: bool) -> None:
    if not top_rel.startswith("features/"):
        top_rel = "features/" + top_rel
    top_path = SRC / top_rel
    if not top_path.is_file():
        sys.exit(f"未找到文件: {top_path}")

    mod_name = top_rel.replace("/", "_").replace(".rs", "").replace("-", "_")
    lib_text = LIB.read_text(encoding="utf-8")
    old = f'include!("{top_rel}");'
    new = (
        f'#[path = "{top_rel}"]\n'
        f"mod {mod_name};\n"
        f"pub(crate) use {mod_name}::*;"
    )

    if inject_use:
        # include! 子文件内容插入到聚合文件 mod 作用域，能看到聚合文件的 use
        # （最小复现已验证 derive 宏可见父 mod use）。因此 use 只注入聚合文件一次，
        # 子文件不注入，避免多个子文件在相同作用域重复导入同名符号（E0252）。
        content = top_path.read_text(encoding="utf-8")
        if re.search(r"^use \w", content, re.MULTILINE):
            print(f"[{top_rel}] 已有 use，跳过注入（幂等）")
        else:
            use_block = extract_lib_use_block()
            if not dry_run:
                top_path.write_text(use_block + content, encoding="utf-8", newline="\n")
            print(f"[{top_rel}] 注入 lib.rs use 块")

    if inject_pub:
        for f in collect_include_files(top_path):
            changed = inject_pub_crate(f)
            if changed:
                print(f"[{f.relative_to(SRC)}] 顶层符号 +{changed} pub(crate)")
            changed_fields = inject_pub_crate_fields(f)
            if changed_fields:
                print(f"[{f.relative_to(SRC)}] 字段 +{changed_fields} pub(crate)")
            changed_methods = inject_pub_crate_methods(f)
            if changed_methods:
                print(f"[{f.relative_to(SRC)}] 方法 +{changed_methods} pub(crate)")

    if not dry_run:
        if old in lib_text:
            lib_text = lib_text.replace(old, new, 1)
            LIB.write_text(lib_text, encoding="utf-8", newline="\n")
            print(f"[lib.rs] {old} → mod {mod_name} + pub(crate) use")
        else:
            print(f"[lib.rs] {old} 已转换，跳过（幂等）")
    print("完成。请 cargo check 验证，剩余跨域引用错误需人工补 pub(crate) 或 use。")


def main() -> None:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("list").set_defaults(func=lambda a: print("\n".join(list_top_includes())))

    p_conv = sub.add_parser("convert")
    p_conv.add_argument("top_rel", help="features/ 下相对路径，如 core/domain.rs（可省略 features/ 前缀）")
    p_conv.add_argument("--no-inject-use", action="store_true", help="不注入 lib.rs use 块")
    p_conv.add_argument("--no-inject-pub", action="store_true", help="不加 pub(crate)")
    p_conv.set_defaults(func=None)

    args = ap.parse_args()
    if args.cmd == "list":
        print("\n".join(list_top_includes()))
    elif args.cmd == "convert":
        convert(args.top_rel, dry_run=False, inject_use=not args.no_inject_use, inject_pub=not args.no_inject_pub)
    else:
        ap.print_help()


if __name__ == "__main__":
    main()
