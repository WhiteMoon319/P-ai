#!/usr/bin/env python3
"""错误驱动的缺失 use 补齐（阶段 3 include→module 辅助）。

用法: python tools/android/fix_missing_uses.py
从 cargo check 错误中提取缺失符号，向对应聚合 mod 补 use。
"""
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
SRC = REPO / "src-tauri" / "src"
LIB = SRC / "lib.rs"

SYMBOL_USE = {
    "Path": "use std::path::Path;",
    "PathBuf": "use std::path::PathBuf;",
    "HashMap": "use std::collections::HashMap;",
    "HashSet": "use std::collections::HashSet;",
    "BTreeMap": "use std::collections::BTreeMap;",
    "VecDeque": "use std::collections::VecDeque;",
    "Pin": "use std::pin::Pin;",
    "DateTime": "use chrono::{DateTime, Utc};",
    "Utc": "use chrono::{DateTime, Utc};",
    "Index": "use tantivy::Index;",
    "Schema": "use tantivy::schema::Schema;",
    "FAST": "use tantivy::schema::FAST;",
    "STORED": "use tantivy::schema::STORED;",
    "IndexRecordOption": "use tantivy::schema::IndexRecordOption;",
    "TextFieldIndexing": "use tantivy::schema::TextFieldIndexing;",
    "TextOptions": "use tantivy::schema::TextOptions;",
    "QueryParser": "use tantivy::QueryParser;",
    "Searcher": "use tantivy::Searcher;",
    "TantivyDocument": "use tantivy::TantivyDocument;",
    "TopDocs": "use tantivy::collector::TopDocs;",
    "Request": "use axum::extract::Request;",
    "Response": "use axum::response::Response;",
}


def run_check() -> str:
    r = subprocess.run(
        ["cargo", "check", "--target", "aarch64-linux-android", "--message-format=short"],
        capture_output=True, text=True, cwd=REPO / "src-tauri",
    )
    return r.stderr


def main() -> int:
    err = run_check()
    fixes = {}
    for line in err.splitlines():
        m = re.match(
            r'^(src[^:]+):\d+:\d+: error.*(?:cannot find type|cannot find value|cannot find macro|use of undeclared type) `([A-Za-z_]+)`',
            line,
        )
        if m:
            f, sym = m.group(1), m.group(2)
            if sym in SYMBOL_USE:
                fixes.setdefault(f, set()).add(sym)
    if not fixes:
        print("无缺失 use 错误")
        return 0
    for f, syms in fixes.items():
        rel = f.replace("\\", "/")
        if rel.startswith("src/"):
            rel = rel[len("src/"):]
        fp = SRC / rel
        # 找聚合 mod：fp 或包含它的顶层 include
        target = fp
        for rel in re.findall(r'#\[path = "([^"]+)"\]', LIB.read_text(encoding='utf-8')):
            agg = (SRC / rel).resolve()
            if agg == fp.resolve():
                target = agg
                break
        s = target.read_text(encoding='utf-8')
        add_lines = []
        for sym in sorted(syms):
            use_line = SYMBOL_USE[sym]
            if use_line in s:
                continue
            add_lines.append(use_line)
        if add_lines:
            block = "\n".join(add_lines) + "\n"
            anchor = "use super::*;"
            if anchor in s:
                s = s.replace(anchor, block + anchor, 1)
            else:
                anchor2 = "use crate::*;"
                if anchor2 in s:
                    s = s.replace(anchor2, block + anchor2, 1)
                else:
                    s = block + s
            target.write_text(s, encoding="utf-8", newline="\n")
            print(f"[{target.relative_to(SRC)}] + {sorted(syms)}")
    return 1


if __name__ == "__main__":
    for _ in range(5):
        if main() == 0:
            break
    sys.exit(0)
