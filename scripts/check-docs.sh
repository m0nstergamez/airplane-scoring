#!/usr/bin/env bash
# Keep the manual in sync: docs/content/de and docs/content/en must have
# the same files, and each pair the same structure (heading levels in order,
# numbered steps, bullets, table rows, code blocks; for diagrams also type,
# line count and number of arrows). Wording may differ.
# The project notes docs/README.md and docs/README.de.md are checked the
# same way.
# German uses Swiss spelling: no "ß" anywhere in docs/, and Swiss
# quotation marks «…» (no „…“ or “…”) in the German manual, the German
# project notes and the glossary.
set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 - <<'PY'
import pathlib, re, sys

root = pathlib.Path("docs/content")
langs = sorted(p.name for p in root.iterdir() if p.is_dir())
files = {l: sorted(p.relative_to(root / l).as_posix() for p in (root / l).rglob("*.md")) for l in langs}

def shape(path):
    s = {"headings": [], "steps": 0, "bullets": 0, "table rows": 0, "code blocks": []}
    block = None  # [language, lines, arrows] of the code block we are in
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("```"):
            if block is None:
                block = [line[3:].strip() or "text", 0, 0]
            else:
                s["code blocks"].append(tuple(block))
                block = None
            continue
        if block is not None:
            if line.strip():
                block[1] += 1
                block[2] += len(re.findall(r"-->|->>|-->>", line))
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

def compare(name, path_a, label_a, path_b, label_b):
    a, b = shape(path_a), shape(path_b)
    for k in a:
        if a[k] != b[k]:
            errors.append(f"{name}: {k} differ ({label_a}: {a[k]}, {label_b}: {b[k]})")

base = langs[0]
for other in langs[1:]:
    for f in sorted(set(files[base]) ^ set(files[other])):
        missing = other if f in files[base] else base
        errors.append(f"{missing}/{f} is missing")
    for f in sorted(set(files[base]) & set(files[other])):
        compare(f, root / base / f, base, root / other / f, other)

notes_en, notes_de = pathlib.Path("docs/README.md"), pathlib.Path("docs/README.de.md")
for f in (notes_en, notes_de):
    if not f.exists():
        errors.append(f"{f} is missing")
if notes_en.exists() and notes_de.exists():
    compare("docs/README", notes_de, "de", notes_en, "en")

for md in sorted(pathlib.Path("docs").rglob("*.md")):
    for n, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
        if "\u00df" in line:
            errors.append(f"{md}:{n}: use Swiss spelling 'ss' instead of the sharp s")
        german = md.parts[1:3] == ("content", "de") or md.name in ("glossary.md", "README.de.md")
        if german and any(q in line for q in "\u201e\u201c\u201d"):
            errors.append(f"{md}:{n}: use Swiss quotation marks \u00ab\u2026\u00bb")

if errors:
    print("Docs check failed:", *errors, sep="\n  ", file=sys.stderr)
    sys.exit(1)
print(f"manual in sync: {', '.join(langs)} x {len(files[base])} files, project notes de/en")
PY
