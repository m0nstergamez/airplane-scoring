#!/usr/bin/env bash
# Keep the manual in sync: docs/content/de and docs/content/en must have
# the same files, and each pair the same structure (heading levels in order,
# numbered steps, bullets, table rows, code blocks). Wording may differ.
set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 - <<'PY'
import pathlib, re, sys

root = pathlib.Path("docs/content")
langs = sorted(p.name for p in root.iterdir() if p.is_dir())
files = {l: sorted(p.relative_to(root / l).as_posix() for p in (root / l).rglob("*.md")) for l in langs}

def shape(path):
    s = {"headings": [], "steps": 0, "bullets": 0, "table rows": 0, "code blocks": 0}
    in_code = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("```"):
            in_code = not in_code
            s["code blocks"] += in_code
            continue
        if in_code:
            continue
        if m := re.match(r"(#+) ", line):
            s["headings"].append(len(m.group(1)))
        elif re.match(r"\s*\d+\. ", line):
            s["steps"] += 1
        elif re.match(r"\s*[-*] ", line):
            s["bullets"] += 1
        elif line.startswith("|"):
            s["table rows"] += 1
    return s

errors = []
base = langs[0]
for other in langs[1:]:
    for f in sorted(set(files[base]) ^ set(files[other])):
        missing = other if f in files[base] else base
        errors.append(f"{missing}/{f} is missing")
    for f in sorted(set(files[base]) & set(files[other])):
        a, b = shape(root / base / f), shape(root / other / f)
        for k in a:
            if a[k] != b[k]:
                errors.append(f"{f}: {k} differ ({base}: {a[k]}, {other}: {b[k]})")

if errors:
    print("Manual out of sync (docs/content):", *errors, sep="\n  ", file=sys.stderr)
    sys.exit(1)
print(f"manual in sync: {', '.join(langs)} x {len(files[base])} files")
PY
