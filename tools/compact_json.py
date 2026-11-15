#!/usr/bin/env python3
"""Remove JSON whitespace without rewriting strings, numbers, or field order.

Defaults to installed assets only. User projects/exports and vendor are not
touched unless their paths are explicitly supplied.
"""
import argparse
import json
import os
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def compact_json(text):
    json.loads(text)  # Refuse malformed input before changing a file.
    output = []
    quoted = False
    escaped = False
    for char in text:
        if quoted:
            output.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
            output.append(char)
        elif char not in " \t\r\n":
            output.append(char)
    return "".join(output) + "\n"


def compact_file(path):
    original = path.read_bytes()
    compact = compact_json(original.decode("utf-8")).encode("utf-8")
    if compact == original:
        return len(original), len(compact), False
    # Stage in the same directory; interrupted writes leave the source intact.
    pending = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as file:
            pending = Path(file.name)
            file.write(compact)
            file.flush()
            os.fsync(file.fileno())
        pending.chmod(path.stat().st_mode & 0o777)
        pending.replace(path)
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)
    return len(original), len(compact), True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path, default=[ROOT / "assets"])
    args = parser.parse_args()
    files = set()
    for path in args.paths:
        if not path.exists():
            parser.error(f"Path does not exist: {path}")
        candidates = path.rglob("*.json") if path.is_dir() else [path]
        files.update(p for p in candidates if p.is_file() and not p.is_symlink())
    before = after = changed = 0
    for path in sorted(files):
        old_size, new_size, modified = compact_file(path)
        before += old_size
        after += new_size
        changed += modified
    percent = (1.0 - after / before) * 100 if before else 0.0
    print(f"Compacted {changed}/{len(files)} JSON files: "
          f"{before:,} -> {after:,} bytes ({percent:.1f}% smaller)")


if __name__ == "__main__":
    main()
