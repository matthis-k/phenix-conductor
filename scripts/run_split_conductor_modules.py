from pathlib import Path

path = Path("scripts/split_conductor_modules.py")
source = path.read_text()
needle = """            elif ch == \"'\":\n                state = \"char\"\n"""
replacement = """            elif ch == \"'\" and (\n                nxt == \"\\\\\" or (i + 2 < len(text) and text[i + 2] == \"'\")\n            ):\n                state = \"char\"\n"""
count = source.count(needle)
if count != 3:
    raise RuntimeError(f"expected 3 Rust apostrophe scanner sites, found {count}")
source = source.replace(needle, replacement)
exec(compile(source, str(path), "exec"), {"__name__": "__main__"})
