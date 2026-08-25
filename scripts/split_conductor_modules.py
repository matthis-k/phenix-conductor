from __future__ import annotations

import subprocess
from pathlib import Path

SOURCE_COMMIT = "8b6c19e2b0d21d1b76aed880ced9e343d59c75c9"

subprocess.run(["git", "fetch", "origin", "main", SOURCE_COMMIT], check=True)
source = subprocess.check_output(
    ["git", "show", f"{SOURCE_COMMIT}:scripts/split_conductor_modules.py"],
    text=True,
)

needle = """            elif ch == \"'\":\n                state = \"char\"\n"""
replacement = """            elif ch == \"'\" and (\n                nxt == \"\\\\\" or (i + 2 < len(text) and text[i + 2] == \"'\")\n            ):\n                state = \"char\"\n"""
count = source.count(needle)
if count != 3:
    raise RuntimeError(f"expected 3 Rust apostrophe scanner sites, found {count}")
source = source.replace(needle, replacement)
exec(compile(source, "scripts/split_conductor_modules.py", "exec"), {"__name__": "__main__"})

Path("flake.nix").write_text(
    subprocess.check_output(["git", "show", "origin/main:flake.nix"], text=True)
)
for artifact in (
    "modules/split-transport.nix",
    "scripts/.worker-395-transport",
    "scripts/run_split_conductor_modules.py",
):
    Path(artifact).unlink(missing_ok=True)
