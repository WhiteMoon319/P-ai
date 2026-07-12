#!/usr/bin/env python3
"""Audit Rust and frontend runtime logging without modifying source files."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


SOURCE_SUFFIXES = {".rs", ".ts", ".tsx", ".vue", ".js", ".mjs"}
IGNORED_PARTS = {"dist", "node_modules", "target", "target-tests", ".git"}
RUST_CALLS = {
    "runtime_log_error": "error",
    "runtime_log_warn": "warn",
    "runtime_log_info": "info",
    "runtime_log_debug": "debug",
    "eprintln": "info",
}
FRONTEND_CALLS = {
    "console.error": "error",
    "console.warn": "warn",
    "console.info": "info",
    "console.debug": "debug",
    "console.log": "info",
}
CALL_PATTERN = re.compile(
    r"(?P<name>runtime_log_(?:error|warn|info|debug)|eprintln|console\.(?:error|warn|info|debug|log))\s*!?\s*\("
)
PREFIX_LEVEL_PATTERN = re.compile(r"\[(ERROR|WARN|INFO|DEBUG|TRACE)\]", re.IGNORECASE)
CHINESE_PATTERN = re.compile(r"[\u3400-\u9fff]")
ERROR_HINT_PATTERN = re.compile(
    r"失败|异常|错误|failed|failure|panic|崩溃|不可用|拒绝|超时|timeout",
    re.IGNORECASE,
)
WARN_HINT_PATTERN = re.compile(
    r"跳过|降级|兜底|重试|忽略|缺少|不存在|fallback|retry|\bskip(?:ped)?\b|\bmissing\b|\bignored\b",
    re.IGNORECASE,
)
DEBUG_HINT_PATTERN = re.compile(
    r"详情|明细|快照|变量|payload|参数|debug|trace|诊断|探测|轮询|心跳|缓存命中",
    re.IGNORECASE,
)
ENGLISH_STATUS_PATTERN = re.compile(r"\bstatus\s*=\s*(?:success|failed|skipped)\b", re.IGNORECASE)
ERROR_CONTEXT_PATTERN = re.compile(
    r"\berr(?:or)?\b|异常|错误|error\.message|error\.stack|\{:\?\}|\{\}|\{err\}|\{error\}",
    re.IGNORECASE,
)
LOOP_PATTERN = re.compile(r"\b(?:for|while|loop)\b")
TASK_PREFIXES = ("[睡眠维护]", "[睡眠]", "[简单记忆回灌]")
LOG_PREFIX_TRANSLATIONS = {
    "[TOOL-DEBUG]": "[工具调试]",
    "[CONFIG]": "[配置]",
    "[MCP Supervisor]": "[MCP监管]",
    "[MEMORY]": "[记忆]",
    "[ARCHIVE-IMPORT]": "[归档导入]",
    "[ARCHIVE-PIPELINE]": "[归档流程]",
    "[TEST]": "[测试]",
    "[BOOTSTRAP]": "[启动]",
    "[PERF]": "[性能]",
    "[LOCALE]": "[语言]",
    "[LIFECYCLE]": "[生命周期]",
    "[THEME]": "[主题]",
    "[UPDATE]": "[自动更新]",
    "[WINDOW]": "[窗口]",
    "[CHAT-QUEUE]": "[聊天队列]",
    "[VIEW]": "[视图]",
    "[RemoteImTab]": "[远程IM]",
    "[TERMINAL]": "[终端]",
    "[AboutTab]": "[关于]",
    "[SHELL]": "[工作区]",
    "[TRAY]": "[托盘]",
    "[ARCHIVES]": "[归档]",
    "[WATCH]": "[监听]",
}
LOG_PHRASE_TRANSLATIONS = {
    "execute_builtin_tool.start": "内置工具执行开始",
    "execute_builtin_tool.ok": "内置工具执行完成",
    "execute_builtin_tool.err": "内置工具执行失败",
    "Parse config failed": "解析配置失败",
    "Parse {label} failed": "解析{label}失败",
    "skip invalid file": "跳过无效文件",
    "vector search failed, fallback to bm25-only path.": "向量搜索失败，降级为仅 BM25 检索。",
    "rerank failed, fallback to non-rerank scoring.": "重排失败，降级为不重排评分。",
    "resolve stored ref failed": "解析已存储引用失败",
    "externalize resolved media failed": "外置已解析媒体失败",
    "externalize media base64 failed": "外置媒体 base64 失败",
    "delete merged source memory failed": "删除已合并来源记忆失败",
    "history_flushed emit skipped": "history_flushed 事件发送跳过",
    "history_flushed emit failed": "history_flushed 事件发送失败",
    "history_flushed emitted": "history_flushed 事件发送完成",
    "terminal approval request not found": "未找到终端审批请求",
    "terminal approval receiver dropped": "终端审批接收端已关闭",
    "skip shell kind=": "跳过 Shell，类型=",
    "not available on this machine": "当前设备不可用",
    "skipping terminal approval listener: not chat window": "跳过终端审批监听：当前不是聊天窗口",
    "startup step failed": "启动步骤失败",
    "wait backend ready failed, continue startup refresh": "等待后端就绪失败，继续执行启动刷新",
    "startup safety gate failed": "启动安全门失败",
    "startup refresh failed": "启动刷新失败",
    "startup lifecycle failed": "启动生命周期失败",
    "mcp_runtime_state_set lock poisoned": "设置 MCP 运行状态时锁中毒",
    "mcp_runtime_state_remove lock poisoned": "移除 MCP 运行状态时锁中毒",
    "mcp_runtime_state_update lock poisoned": "更新 MCP 运行状态时锁中毒",
    "trace={} begin api={}": "trace={} 开始，api={}",
    "Codex mode detected: skipping API key validation; using empty candidate API key for selectedApiConfig=": "检测到 Codex 模式：跳过 API Key 校验，使用空候选 API Key，selectedApiConfig=",
    "fetch model metadata failed": "获取模型元数据失败",
    "open provider docs failed": "打开供应商文档失败",
}


@dataclass
class Finding:
    path: str
    line: int
    call: str
    level: str
    inferred_level: str
    message: str
    issues: list[str]


@dataclass
class SourceLogEntry:
    sequence: int
    path: str
    line: int
    start: int
    end: int
    call: str
    source_sha256: str
    file_sha256: str
    original_source: str
    code: str


def read_source_text(path: Path) -> str:
    return path.read_bytes().decode("utf-8", errors="replace")


def iter_source_files(root: Path) -> Iterable[Path]:
    for base in (root / "src-tauri" / "src", root / "src"):
        if not base.exists():
            continue
        for path in base.rglob("*"):
            if not path.is_file() or path.suffix.lower() not in SOURCE_SUFFIXES:
                continue
            if any(part in IGNORED_PARTS or part.startswith("target-") for part in path.parts):
                continue
            yield path


def mask_non_code(text: str, suffix: str) -> str:
    chars = list(text)
    index = 0
    while index < len(text):
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = len(text) if end < 0 else end
            for pos in range(index, end):
                chars[pos] = " "
            index = end
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            end = len(text) if end < 0 else end + 2
            for pos in range(index, end):
                if chars[pos] != "\n":
                    chars[pos] = " "
            index = end
            continue
        raw_match = re.match(r"r(?P<hashes>#{0,16})\"", text[index:])
        if raw_match:
            hashes = raw_match.group("hashes")
            closing = '"' + hashes
            end = text.find(closing, index + raw_match.end())
            end = len(text) if end < 0 else end + len(closing)
            for pos in range(index, end):
                if chars[pos] != "\n":
                    chars[pos] = " "
            index = end
            continue
        quote_chars = {'"', "`"} if suffix == ".rs" else {'"', "'", "`"}
        if text[index] in quote_chars:
            quote = text[index]
            pos = index + 1
            escaped = False
            while pos < len(text):
                char = text[pos]
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    pos += 1
                    break
                pos += 1
            for masked_pos in range(index, pos):
                if chars[masked_pos] != "\n":
                    chars[masked_pos] = " "
            index = pos
            continue
        index += 1
    return "".join(chars)


def iter_log_matches(text: str, suffix: str) -> Iterable[re.Match[str]]:
    masked = mask_non_code(text, suffix)
    rust_raw_spans: list[tuple[int, int]] = []
    if suffix == ".rs":
        rust_raw_spans = [
            (match.start(), match.end())
            for match in re.finditer(r'(?s)r(?P<hashes>#{0,16})\".*?\"(?P=hashes)', text)
        ]
    candidates = {match.start(): match for match in CALL_PATTERN.finditer(masked)}
    if suffix == ".rs":
        for match in CALL_PATTERN.finditer(text):
            if match.group("name") == "eprintln":
                candidates.setdefault(match.start(), match)
    for match in (candidates[start] for start in sorted(candidates)):
        declaration_prefix = text[max(0, match.start() - 16):match.start()]
        if re.search(r"\bfn\s+$", declaration_prefix):
            continue
        if any(start <= match.start() < end for start, end in rust_raw_spans):
            continue
        if match.group("name") == "eprintln":
            line_start = text.rfind("\n", 0, match.start()) + 1
            if "macro_rules!" in text[line_start:match.start()]:
                continue
        if "macro_rules! eprintln" in text[max(0, match.start() - 160):match.start()]:
            continue
        yield match


def extract_call(text: str, start: int) -> str:
    depth = 0
    quote = ""
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if quote:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = ""
            continue
        if char in {'"', "'", "`"}:
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return text[start:index + 1]
    return text[start:text.find("\n", start) if "\n" in text[start:] else len(text)]


def compact_message(call_text: str, limit: int = 220) -> str:
    compact = re.sub(r"\s+", " ", call_text).strip()
    return compact if len(compact) <= limit else compact[: limit - 3] + "..."


def single_line_code(source: str) -> str:
    output: list[str] = []
    quote = ""
    escaped = False
    pending_space = False
    for char in source.strip():
        if quote:
            output.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = ""
            continue
        if char in {'"', "'", "`"}:
            if pending_space and output and output[-1] not in "([{":
                output.append(" ")
            pending_space = False
            quote = char
            output.append(char)
            continue
        if char.isspace():
            pending_space = True
            continue
        if pending_space and output and output[-1] not in "([{,":
            output.append(" ")
        pending_space = False
        output.append(char)
    return "".join(output)


def infer_level(level: str, message: str) -> str:
    prefix = PREFIX_LEVEL_PATTERN.search(message)
    if prefix:
        return prefix.group(1).lower()
    if WARN_HINT_PATTERN.search(message):
        return "warn"
    if ERROR_HINT_PATTERN.search(message):
        return "error"
    if DEBUG_HINT_PATTERN.search(message):
        return "debug"
    return level


def nearby_loop(text: str, offset: int) -> bool:
    line_start = text.rfind("\n", 0, offset)
    context_start = max(0, text.rfind("\n", 0, max(0, line_start - 800)))
    context = text[context_start:offset]
    return bool(LOOP_PATTERN.search(context))


def audit_file(root: Path, path: Path) -> list[Finding]:
    text = read_source_text(path)
    findings: list[Finding] = []
    for match in iter_log_matches(text, path.suffix.lower()):
        call = match.group("name")
        level = RUST_CALLS.get(call, FRONTEND_CALLS.get(call, "info"))
        call_text = extract_call(text, match.end() - 1)
        message = compact_message(call_text)
        inferred_level = infer_level(level, message)
        issues: list[str] = []
        if call == "eprintln":
            issues.append("eprintln_effective_info")
        if inferred_level != level:
            issues.append(f"level_mismatch:{level}->{inferred_level}")
        if not CHINESE_PATTERN.search(message):
            issues.append("message_without_chinese")
        if ENGLISH_STATUS_PATTERN.search(message):
            issues.append("english_status_expression")
        if level == "error" and not ERROR_CONTEXT_PATTERN.search(message):
            issues.append("error_without_context")
        if nearby_loop(text, match.start()) and level in {"info", "warn"}:
            issues.append("possible_high_frequency_log")
        if any(prefix in message for prefix in TASK_PREFIXES):
            if not re.search(r"开始|完成|跳过|失败", message):
                issues.append("task_log_without_standard_state")
        line = text.count("\n", 0, match.start()) + 1
        findings.append(Finding(
            path=path.relative_to(root).as_posix(),
            line=line,
            call=call,
            level=level,
            inferred_level=inferred_level,
            message=message,
            issues=issues,
        ))
    return findings


def extract_source_entries(root: Path) -> list[SourceLogEntry]:
    entries: list[SourceLogEntry] = []
    for path in iter_source_files(root):
        raw_bytes = path.read_bytes()
        text = raw_bytes.decode("utf-8", errors="replace")
        file_sha256 = hashlib.sha256(raw_bytes).hexdigest()
        for match in iter_log_matches(text, path.suffix.lower()):
            call_body = extract_call(text, match.end() - 1)
            source = text[match.start():match.end() - 1] + call_body
            start = match.start()
            end = start + len(source)
            entries.append(SourceLogEntry(
                sequence=len(entries) + 1,
                path=path.relative_to(root).as_posix(),
                line=text.count("\n", 0, start) + 1,
                start=start,
                end=end,
                call=match.group("name"),
                source_sha256=hashlib.sha256(source.encode("utf-8")).hexdigest(),
                file_sha256=file_sha256,
                original_source=source,
                code=single_line_code(source),
            ))
    return entries


def write_source_export(root: Path, code_txt: Path, index_json: Path) -> None:
    entries = extract_source_entries(root)
    code_txt.parent.mkdir(parents=True, exist_ok=True)
    index_json.parent.mkdir(parents=True, exist_ok=True)
    code_txt.write_text("\n".join(item.code for item in entries) + "\n", encoding="utf-8")
    index_json.write_text(
        json.dumps(
            [{key: value for key, value in asdict(item).items() if key != "code"} for item in entries],
            ensure_ascii=False,
            indent=2,
        ) + "\n",
        encoding="utf-8",
    )


def materialize_preserving_layout(item: dict[str, object], edited_code: str) -> str:
    original = str(item["original_source"])
    original_call = str(item["call"])
    edited_call_match = CALL_PATTERN.match(edited_code)
    if not edited_call_match:
        raise ValueError(f"无法读取编辑后的日志调用：{item['path']}:{item['line']}")
    edited_call = edited_call_match.group("name")
    next_source = original
    if original_call == "eprintln" and edited_call != "eprintln":
        open_paren = original.find("(")
        next_source = f"{edited_call}(format!{original[open_paren:]})"
    elif original_call != edited_call:
        next_source = re.sub(
            r"^(?:runtime_log_(?:error|warn|info|debug)|console\.(?:error|warn|info|debug|log))",
            edited_call,
            original,
            count=1,
        )
    next_source = re.sub(r"status\s*=\s*success", "状态=完成", next_source, flags=re.IGNORECASE)
    next_source = re.sub(r"status\s*=\s*failed", "状态=失败", next_source, flags=re.IGNORECASE)
    next_source = re.sub(r"status\s*=\s*skipped", "状态=跳过", next_source, flags=re.IGNORECASE)
    for source_prefix, target_prefix in LOG_PREFIX_TRANSLATIONS.items():
        next_source = next_source.replace(source_prefix, target_prefix)
    for source_phrase, target_phrase in LOG_PHRASE_TRANSLATIONS.items():
        next_source = next_source.replace(source_phrase, target_phrase)
    return next_source


def apply_source_export(root: Path, code_txt: Path, index_json: Path, write: bool) -> None:
    code_lines = code_txt.read_text(encoding="utf-8").splitlines()
    raw_index = json.loads(index_json.read_text(encoding="utf-8"))
    if len(code_lines) != len(raw_index):
        raise ValueError(f"TXT 行数与索引数量不一致：txt={len(code_lines)} index={len(raw_index)}")
    if any(not line.strip() for line in code_lines):
        raise ValueError("TXT 中存在空行")
    allowed_prefixes = tuple(f"{name}(" for name in FRONTEND_CALLS) + tuple(
        f"{name}(" for name in RUST_CALLS if name != "eprintln"
    ) + ("eprintln!(",)
    if any(not line.lstrip().startswith(allowed_prefixes) for line in code_lines):
        raise ValueError("TXT 中存在非日志代码行")

    by_path: dict[str, list[tuple[dict[str, object], str]]] = defaultdict(list)
    for item, code in zip(raw_index, code_lines, strict=True):
        by_path[str(item["path"])].append((item, code))

    rewritten: dict[Path, bytes] = {}
    for relative_path, replacements in by_path.items():
        path = root / relative_path
        raw_bytes = path.read_bytes()
        current_file_hash = hashlib.sha256(raw_bytes).hexdigest()
        expected_file_hashes = {str(item["file_sha256"]) for item, _ in replacements}
        if expected_file_hashes != {current_file_hash}:
            raise ValueError(f"文件已变化，拒绝回填：{relative_path}")
        original_text = raw_bytes.decode("utf-8", errors="replace")
        parts: list[str] = []
        applied_ranges: list[tuple[int, int, str]] = []
        cursor = 0
        output_length = 0
        for item, code in replacements:
            start = int(item["start"])
            end = int(item["end"])
            original_source = str(item["original_source"])
            current_source = original_text[start:end]
            if current_source != original_source:
                raise ValueError(f"原文不匹配：{relative_path}:{item['line']}")
            if hashlib.sha256(current_source.encode("utf-8")).hexdigest() != item["source_sha256"]:
                raise ValueError(f"原文哈希不匹配：{relative_path}:{item['line']}")
            replacement_source = materialize_preserving_layout(item, code)
            if single_line_code(replacement_source) != code:
                raise ValueError(f"回填后代码不匹配：{relative_path}:{item['line']}")
            unchanged = original_text[cursor:start]
            parts.append(unchanged)
            output_length += len(unchanged)
            applied_ranges.append((output_length, len(replacement_source), original_source))
            parts.append(replacement_source)
            output_length += len(replacement_source)
            cursor = end
        parts.append(original_text[cursor:])
        next_text = "".join(parts)
        reverse_text = next_text
        for start, replacement_length, original_source in reversed(applied_ranges):
            reverse_text = reverse_text[:start] + original_source + reverse_text[start + replacement_length:]
        if reverse_text != original_text:
            raise ValueError(f"反向还原失败：{relative_path}")
        rewritten[path] = next_text.encode("utf-8")

    if write:
        for path, content in rewritten.items():
            path.write_bytes(content)
    action = "正式回填" if write else "dry-run"
    print(f"{action}通过：logs={len(code_lines)} files={len(rewritten)} reversible=true")


def normalize_exported_code(input_txt: Path, index_json: Path, output_txt: Path) -> None:
    code_lines = input_txt.read_text(encoding="utf-8").splitlines()
    raw_index = json.loads(index_json.read_text(encoding="utf-8"))
    if len(code_lines) != len(raw_index):
        raise ValueError(f"TXT 行数与索引数量不一致：txt={len(code_lines)} index={len(raw_index)}")
    normalized: list[str] = []
    change_counts: Counter[str] = Counter()
    for code, item in zip(code_lines, raw_index, strict=True):
        call = str(item["call"])
        source_path = str(item["path"])
        current_level = RUST_CALLS.get(call, FRONTEND_CALLS.get(call, "info"))
        inferred_level = infer_level(current_level, code)
        target_level = inferred_level if current_level == "info" else current_level
        if target_level == "trace":
            target_level = "info"
        next_code = code
        if call == "eprintln" and source_path.startswith("src-tauri/src/bin/"):
            next_code = code
        elif call == "eprintln":
            inner = code[len("eprintln!("):-1]
            next_code = f"runtime_log_{target_level}(format!({inner}))"
        elif call.startswith("runtime_log_") and current_level != target_level:
            next_code = re.sub(r"^runtime_log_(?:error|warn|info|debug)", f"runtime_log_{target_level}", code, count=1)
        elif call.startswith("console.") and current_level != target_level:
            next_code = re.sub(r"^console\.(?:error|warn|info|debug|log)", f"console.{target_level}", code, count=1)
        next_code = re.sub(r"status\s*=\s*success", "状态=完成", next_code, flags=re.IGNORECASE)
        next_code = re.sub(r"status\s*=\s*failed", "状态=失败", next_code, flags=re.IGNORECASE)
        next_code = re.sub(r"status\s*=\s*skipped", "状态=跳过", next_code, flags=re.IGNORECASE)
        for source_prefix, target_prefix in LOG_PREFIX_TRANSLATIONS.items():
            next_code = next_code.replace(source_prefix, target_prefix)
        for source_phrase, target_phrase in LOG_PHRASE_TRANSLATIONS.items():
            next_code = next_code.replace(source_phrase, target_phrase)
        normalized.append(next_code)
        if next_code != code:
            change_counts[f"{call}->{next_code.split('(', 1)[0]}"] += 1
    output_txt.parent.mkdir(parents=True, exist_ok=True)
    output_txt.write_text("\n".join(normalized) + "\n", encoding="utf-8")
    print(f"规范化 TXT 已生成：logs={len(normalized)} changed={sum(change_counts.values())}")
    for key, count in change_counts.most_common():
        print(f"{key}: {count}")


def restore_from_index(root: Path, index_json: Path) -> None:
    raw_index = json.loads(index_json.read_text(encoding="utf-8"))
    by_path: dict[str, list[dict[str, object]]] = defaultdict(list)
    for item in raw_index:
        by_path[str(item["path"])].append(item)
    restored: dict[Path, bytes] = {}
    for relative_path, entries in by_path.items():
        path = root / relative_path
        current_text = read_source_text(path)
        matches = list(iter_log_matches(current_text, path.suffix.lower()))
        if len(matches) != len(entries):
            raise ValueError(
                f"恢复前日志数量不一致：{relative_path} expected={len(entries)} actual={len(matches)}"
            )
        next_text = current_text
        for match, item in reversed(list(zip(matches, entries, strict=True))):
            call_body = extract_call(current_text, match.end() - 1)
            source = current_text[match.start():match.end() - 1] + call_body
            next_text = next_text[:match.start()] + str(item["original_source"]) + next_text[match.start() + len(source):]
        restored_bytes = next_text.encode("utf-8")
        expected_hashes = {str(item["file_sha256"]) for item in entries}
        if expected_hashes != {hashlib.sha256(restored_bytes).hexdigest()}:
            raise ValueError(f"恢复后文件哈希不匹配：{relative_path}")
        restored[path] = restored_bytes
    for path, content in restored.items():
        path.write_bytes(content)
    print(f"索引恢复完成：logs={len(raw_index)} files={len(restored)} hash_match=true")


def render_markdown(findings: list[Finding]) -> str:
    level_counts = Counter(item.level for item in findings)
    issue_counts = Counter(issue for item in findings for issue in item.issues)
    file_counts = Counter(item.path for item in findings)
    suspicious = [item for item in findings if item.issues]
    lines = [
        "# 运行日志静态审计报告",
        "",
        "> 本报告由 `scripts/audit_runtime_logs.py` 静态扫描生成，只用于定位候选项，等级判断仍需结合业务语义复核。",
        "",
        "## 汇总",
        "",
        f"- 日志调用总数：{len(findings)}",
        f"- 存在候选问题的调用：{len(suspicious)}",
        f"- Rust `eprintln!`（实际进入 info）：{sum(1 for item in findings if item.call == 'eprintln')}",
        "- 当前等级分布：" + "、".join(f"{key}={value}" for key, value in sorted(level_counts.items())),
        "",
        "## 问题分布",
        "",
        "| 规则 | 数量 |",
        "|---|---:|",
    ]
    for issue, count in issue_counts.most_common():
        lines.append(f"| `{issue}` | {count} |")
    lines.extend(["", "## 日志最多的文件", "", "| 文件 | 数量 |", "|---|---:|"])
    for path, count in file_counts.most_common(30):
        lines.append(f"| `{path}` | {count} |")
    lines.extend(["", "## 候选明细", ""])
    grouped: dict[str, list[Finding]] = defaultdict(list)
    for item in suspicious:
        grouped[item.path].append(item)
    for path in sorted(grouped):
        lines.append(f"### `{path}`")
        lines.append("")
        for item in grouped[path]:
            issues = ", ".join(f"`{issue}`" for issue in item.issues)
            escaped = item.message.replace("|", "\\|")
            lines.append(f"- L{item.line} `{item.call}` {issues}: `{escaped}`")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--markdown", type=Path, help="Write the Markdown report to this path.")
    parser.add_argument("--json", type=Path, help="Write machine-readable findings to this path.")
    parser.add_argument("--code-txt", type=Path, help="Write one pure single-line log expression per line.")
    parser.add_argument("--index-json", type=Path, help="Write the source index for --code-txt.")
    parser.add_argument("--apply-code-txt", type=Path, help="Validate or apply an edited pure-code TXT.")
    parser.add_argument("--apply-index-json", type=Path, help="Source index used for edited TXT validation.")
    parser.add_argument("--write", action="store_true", help="Write validated replacements to source files.")
    parser.add_argument("--normalize-code-txt", type=Path, help="Write a normalized pure-code TXT without touching source.")
    parser.add_argument("--restore-index-json", type=Path, help="Restore exact original log expressions from an index.")
    args = parser.parse_args()
    root = args.root.resolve()
    should_audit = bool(args.markdown or args.json) or not any((
        args.code_txt,
        args.index_json,
        args.apply_code_txt,
        args.apply_index_json,
    ))
    if should_audit:
        findings = [item for path in iter_source_files(root) for item in audit_file(root, path)]
        markdown = render_markdown(findings)
        if args.markdown:
            args.markdown.parent.mkdir(parents=True, exist_ok=True)
            args.markdown.write_text(markdown, encoding="utf-8")
        else:
            print(markdown, end="")
        if args.json:
            args.json.parent.mkdir(parents=True, exist_ok=True)
            args.json.write_text(
                json.dumps([asdict(item) for item in findings], ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
    if bool(args.code_txt) != bool(args.index_json):
        parser.error("--code-txt and --index-json must be provided together")
    if args.code_txt and args.index_json:
        write_source_export(root, args.code_txt, args.index_json)
    if bool(args.apply_code_txt) != bool(args.apply_index_json):
        parser.error("--apply-code-txt and --apply-index-json must be provided together")
    if args.write and not args.apply_code_txt:
        parser.error("--write requires --apply-code-txt and --apply-index-json")
    if args.apply_code_txt and args.apply_index_json:
        apply_source_export(root, args.apply_code_txt, args.apply_index_json, args.write)
    if args.normalize_code_txt:
        if not args.code_txt or not args.index_json:
            parser.error("--normalize-code-txt requires --code-txt and --index-json")
        normalize_exported_code(args.code_txt, args.index_json, args.normalize_code_txt)
    if args.restore_index_json:
        restore_from_index(root, args.restore_index_json)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
