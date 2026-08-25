from __future__ import annotations

import re
from collections import defaultdict
from pathlib import Path

MAX_CHUNK = 18_000


def matching_brace(text: str, opening: int) -> int:
    depth = 0
    state = "code"
    i = opening
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "code":
            if ch == '"':
                state = "string"
            elif ch == "'":
                state = "char"
            elif ch == "/" and nxt == "/":
                state = "line"
                i += 1
            elif ch == "/" and nxt == "*":
                state = "block"
                i += 1
            elif ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return i
        elif state == "string":
            if ch == "\\":
                i += 1
            elif ch == '"':
                state = "code"
        elif state == "char":
            if ch == "\\":
                i += 1
            elif ch == "'":
                state = "code"
        elif state == "line":
            if ch == "\n":
                state = "code"
        elif state == "block" and ch == "*" and nxt == "/":
            state = "code"
            i += 1
        i += 1
    raise RuntimeError("unmatched brace")


def code_brace(text: str, start: int) -> int:
    state = "code"
    i = start
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "code":
            if ch == '"':
                state = "string"
            elif ch == "'":
                state = "char"
            elif ch == "/" and nxt == "/":
                state = "line"
                i += 1
            elif ch == "/" and nxt == "*":
                state = "block"
                i += 1
            elif ch == "{":
                return i
        elif state == "string":
            if ch == "\\":
                i += 1
            elif ch == '"':
                state = "code"
        elif state == "char":
            if ch == "\\":
                i += 1
            elif ch == "'":
                state = "code"
        elif state == "line":
            if ch == "\n":
                state = "code"
        elif state == "block" and ch == "*" and nxt == "/":
            state = "code"
            i += 1
        i += 1
    raise RuntimeError("function body brace not found")


def depth_zero_line_starts(text: str) -> set[int]:
    starts = {0}
    depth = 0
    state = "code"
    i = 0
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "code":
            if ch == '"':
                state = "string"
            elif ch == "'":
                state = "char"
            elif ch == "/" and nxt == "/":
                state = "line"
                i += 1
            elif ch == "/" and nxt == "*":
                state = "block"
                i += 1
            elif ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
        elif state == "string":
            if ch == "\\":
                i += 1
            elif ch == '"':
                state = "code"
        elif state == "char":
            if ch == "\\":
                i += 1
            elif ch == "'":
                state = "code"
        elif state == "line":
            if ch == "\n":
                state = "code"
        elif state == "block" and ch == "*" and nxt == "/":
            state = "code"
            i += 1
        if ch == "\n" and depth == 0 and state in {"code", "line"}:
            starts.add(i + 1)
        i += 1
    return starts


def attach_attributes(text: str, start: int) -> int:
    line_start = text.rfind("\n", 0, start) + 1
    cursor = line_start
    while cursor > 0:
        prev_end = cursor - 1
        prev_start = text.rfind("\n", 0, prev_end) + 1
        line = text[prev_start:prev_end].strip()
        if line.startswith("#[") or line.startswith("///") or line.startswith("//!"):
            cursor = prev_start
            continue
        break
    return cursor


def function_items(text: str, indent: str) -> list[tuple[int, int, str, str]]:
    zero_starts = depth_zero_line_starts(text)
    result = []
    pattern = re.compile(
        rf"^{re.escape(indent)}(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)",
        re.MULTILINE,
    )
    for match in pattern.finditer(text):
        line_start = text.rfind("\n", 0, match.start()) + 1
        if line_start not in zero_starts:
            continue
        start = attach_attributes(text, match.start())
        opening = code_brace(text, match.end())
        end = matching_brace(text, opening) + 1
        result.append((start, end, match.group(1), text[start:end]))
    return result


def pack_groups(
    items: list[tuple[str, str]],
    classifier,
    directory: Path,
) -> list[Path]:
    grouped: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for name, source in items:
        grouped[classifier(name)].append((name, source))

    written = []
    directory.mkdir(parents=True, exist_ok=True)
    for group in sorted(grouped):
        chunks: list[list[str]] = [[]]
        size = 0
        for _, source in grouped[group]:
            source_size = len(source.encode()) + 2
            if chunks[-1] and size + source_size > MAX_CHUNK:
                chunks.append([])
                size = 0
            chunks[-1].append(source.strip("\n"))
            size += source_size
        for index, chunk in enumerate(chunks, start=1):
            suffix = "" if index == 1 else f"_{index}"
            path = directory / f"{group}{suffix}.rs"
            path.write_text("\n\n".join(chunk) + "\n")
            written.append(path)
    return written


def relative_include(owner: Path, child: Path, indent: str = "") -> str:
    relative = child.relative_to(owner.parent).as_posix()
    return f'{indent}include!("{relative}");'


def split_impl_methods(path: Path, header: str, directory: Path, classifier) -> None:
    text = path.read_text()
    start = text.find(header)
    if start < 0:
        raise RuntimeError(f"{path}: missing {header}")
    opening = text.find("{", start + len(header))
    closing = matching_brace(text, opening)
    body = text[opening + 1 : closing]
    methods = function_items(body, "    ")
    if not methods:
        raise RuntimeError(f"{path}: no methods found for {header}")

    grouped_items = [(name, source) for _, _, name, source in methods]
    files = pack_groups(grouped_items, classifier, directory)

    residual = body
    for method_start, method_end, _, _ in reversed(methods):
        residual = residual[:method_start] + residual[method_end:]
    residual = residual.strip("\n")
    includes = "\n".join(relative_include(path, child, "    ") for child in files)
    replacement = "\n"
    if residual.strip():
        replacement += residual.rstrip() + "\n\n"
    replacement += includes + "\n"
    path.write_text(text[: opening + 1] + replacement + text[closing:])


def split_free_functions(path: Path, directory: Path, classifier, excluded: set[str] | None = None) -> None:
    text = path.read_text()
    excluded = excluded or set()
    functions = [item for item in function_items(text, "") if item[2] not in excluded]
    if not functions:
        return
    files = pack_groups([(name, source) for _, _, name, source in functions], classifier, directory)
    for start, end, _, _ in reversed(functions):
        text = text[:start] + text[end:]
    anchor = text.rfind("#[cfg(test)]")
    includes = "\n".join(relative_include(path, child) for child in files) + "\n\n"
    if anchor < 0:
        text = text.rstrip() + "\n\n" + includes
    else:
        text = text[:anchor] + includes + text[anchor:]
    path.write_text(text)


def split_test_module(path: Path, directory: Path, classifier) -> None:
    text = path.read_text()
    marker = "#[cfg(test)]\nmod tests {"
    start = text.rfind(marker)
    if start < 0:
        return
    opening = text.find("{", start + len("#[cfg(test)]\nmod tests "))
    closing = matching_brace(text, opening)
    body = text[opening + 1 : closing]
    functions = function_items(body, "    ")
    tests = []
    for item in functions:
        _, _, name, source = item
        if "#[test]" in source or "#[tokio::test" in source:
            tests.append(item)
    if not tests:
        return
    files = pack_groups([(name, source) for _, _, name, source in tests], classifier, directory)
    residual = body
    for item_start, item_end, _, _ in reversed(tests):
        residual = residual[:item_start] + residual[item_end:]
    includes = "\n".join(relative_include(path, child, "    ") for child in files)
    replacement = "\n" + residual.strip("\n").rstrip() + "\n\n" + includes + "\n"
    path.write_text(text[: opening + 1] + replacement + text[closing:])


def extract_span(path: Path, start_marker: str, end_header: str, destination: Path) -> None:
    text = path.read_text()
    marker = text.find(start_marker)
    if marker < 0:
        raise RuntimeError(f"{path}: missing {start_marker}")
    start = text.rfind("\n", 0, marker) + 1
    while start > 0:
        previous_end = start - 1
        previous_start = text.rfind("\n", 0, previous_end) + 1
        previous = text[previous_start:previous_end].strip()
        if previous.startswith("#[") or previous.startswith("///"):
            start = previous_start
        else:
            break
    end_start = text.find(end_header, marker)
    if end_start < 0:
        raise RuntimeError(f"{path}: missing {end_header}")
    opening = code_brace(text, end_start + len(end_header))
    end = matching_brace(text, opening) + 1
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(text[start:end].strip("\n") + "\n")
    include = relative_include(path, destination)
    path.write_text(text[:start] + include + text[end:])


def runtime_group(name: str) -> str:
    if any(k in name for k in ("config", "revision", "callable_descriptors", "routing_profiles", "skill_descriptors", "context_catalog")):
        return "configuration"
    if "session" in name or "rebase" in name:
        return "sessions"
    if any(k in name for k in ("workspace", "lease", "checkpoint", "read_set", "diagnostic", "file_observation")):
        return "workspace"
    if any(k in name for k in ("orchestration", "synthesis", "failure", "attempt", "retry", "node")):
        return "orchestration"
    if any(k in name for k in ("provider", "backend", "resolve", "route", "model", "invocation", "prepare")):
        return "invocation"
    if any(k in name for k in ("cancel", "interrupt", "complete", "fail", "state", "termination")):
        return "lifecycle"
    if any(k in name for k in ("event", "subscribe", "unsubscribe", "emit")):
        return "events"
    if any(k in name for k in ("tool", "policy", "callable", "sandbox")):
        return "tooling"
    if any(k in name for k in ("debug", "snapshot", "redact", "conversation")):
        return "debug"
    if any(k in name for k in ("execution", "submit", "child", "root", "output")):
        return "executions"
    if any(k in name for k in ("journal", "restore", "new", "bind")):
        return "runtime"
    return "support"


def runtime_test_group(name: str) -> str:
    return runtime_group(name)


def server_group(name: str) -> str:
    if any(k in name for k in ("serve", "read_request", "handle_message", "respond")):
        return "protocol"
    if any(k in name for k in ("submit", "start_callable", "execution_group", "pending")):
        return "scheduling"
    if any(k in name for k in ("workspace", "checkpoint", "lease")):
        return "workspace"
    if any(k in name for k in ("backend", "catalog", "auth")):
        return "backends"
    if any(k in name for k in ("session", "cancel")):
        return "lifecycle"
    if any(k in name for k in ("persist", "store", "lock_runtime")):
        return "persistence"
    if "debug" in name:
        return "debug"
    if any(k in name for k in ("new", "load", "install", "register", "runtime")):
        return "setup"
    return "support"


def server_free_group(name: str) -> str:
    if any(k in name for k in ("execution", "worker", "enqueue", "drive", "scope")):
        return "workers"
    if any(k in name for k in ("protocol", "map_", "error", "state_token")):
        return "protocol_errors"
    if any(k in name for k in ("frontend", "disconnect")):
        return "frontend"
    if any(k in name for k in ("persist", "checkpoint", "workspace")):
        return "workspace"
    return "support"


def relational_group(name: str) -> str:
    if any(k in name for k in ("migrate", "migration", "metadata", "initialize", "database")):
        return "schema"
    if name == "event_type" or name.startswith("insert_") or name.startswith("serialize_"):
        return "write_events"
    if name.startswith("load_") or name.startswith("parse_") or name.startswith("read_"):
        return "read_events"
    if any(k in name for k in ("objective", "criterion")):
        return "objectives"
    if any(k in name for k in ("context", "injection", "reference")):
        return "context"
    if any(k in name for k in ("target", "model", "inference")):
        return "targets"
    if any(k in name for k in ("authority", "repository", "filesystem", "network")):
        return "authority"
    if any(k in name for k in ("state", "token", "termination", "kind")):
        return "tokens"
    if any(k in name for k in ("json", "value", "number")):
        return "values"
    if any(k in name for k in ("sql_", "invalid")):
        return "sql"
    return "events"


def relational_test_group(name: str) -> str:
    if "context" in name:
        return "context"
    if any(k in name for k in ("objective", "plan")):
        return "planning"
    if any(k in name for k in ("retry", "attempt", "failure", "orchestration")):
        return "orchestration"
    if any(k in name for k in ("workspace", "file", "patch", "language")):
        return "workspace"
    if any(k in name for k in ("round", "restore", "load", "save", "schema", "reject")):
        return "storage"
    return "events"


root = Path("rust/crates/phenix-conductor/src")
lib = root / "lib.rs"

# Keep the crate root focused on declarations and public API. These spans stay in
# the same lexical scope through include!, so this is structural only.
extract_span(
    lib,
    "pub struct ConfigRevisionFingerprint",
    "impl From<PlanError> for ConductorError",
    root / "runtime/error.rs",
)
extract_span(
    lib,
    "struct SessionRecord",
    "struct ExecutionRecord",
    root / "runtime/state_records.rs",
)
extract_span(
    lib,
    "pub struct CompiledConfiguration",
    "impl CompiledConfiguration",
    root / "runtime/configuration_types.rs",
)
extract_span(
    lib,
    "pub struct ResolvedInvocation",
    "impl PreparedInvocation",
    root / "runtime/invocation_types.rs",
)

split_impl_methods(lib, "impl ConductorRuntime ", root / "runtime", runtime_group)
split_free_functions(lib, root / "runtime/helpers", runtime_group)
split_test_module(lib, root / "runtime/tests", runtime_test_group)

server = root / "server_base.rs"
split_impl_methods(server, "impl ConductorServer ", root / "server_base", server_group)
split_free_functions(server, root / "server_base/helpers", server_free_group)
split_test_module(server, root / "server_base/tests", server_group)

relational = root / "persistence/relational.rs"
split_free_functions(relational, root / "persistence/relational", relational_group)
split_test_module(relational, root / "persistence/relational/tests", relational_test_group)

for path in (lib, server, relational):
    size = path.stat().st_size
    if size > 35_000:
        raise RuntimeError(f"{path}: facade remains too large at {size} bytes")

for directory in (
    root / "runtime",
    root / "server_base",
    root / "persistence/relational",
):
    for generated in directory.rglob("*.rs"):
        if generated.stat().st_size > 30_000:
            raise RuntimeError(f"{generated}: generated file is still too large")
