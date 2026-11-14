#!/usr/bin/env bash
# Render every ```mermaid diagram under docs/ with Mermaid in headless
# Chrome. Fails on syntax errors (which GitHub would show as a broken box)
# and saves a gallery for a visual check: target/shots/mermaid.png
# Needs Chrome and network access (Mermaid is loaded from jsDelivr).
# Env: CHROME=/path/to/chrome
set -euo pipefail
cd "$(dirname "$0")/.."
MERMAID_URL="https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.esm.min.mjs"
out=target/shots
mkdir -p "$out"

chrome=$(scripts/find-chrome.sh) || exit 2

count=$(python3 - "$out/mermaid.html" "$MERMAID_URL" <<'PY'
import json, pathlib, re, sys
blocks = []
for md in sorted(pathlib.Path("docs").rglob("*.md")):
    lines = md.read_text(encoding="utf-8").splitlines()
    i = 0
    while i < len(lines):
        if lines[i].strip() == "```mermaid":
            start = i + 1
            j = start
            while not lines[j].startswith("```"):
                j += 1
            blocks.append({"src": f"{md}:{start}", "code": "\n".join(lines[start:j])})
            i = j
        i += 1
html = """<!doctype html><meta charset="utf-8">
<style>body{font:14px system-ui;margin:16px;background:#fff}section{border-top:1px solid #ccc;padding:8px 0}
h3{font:600 13px monospace;color:#555;margin:4px 0}.err{color:#b00;white-space:pre-wrap}svg{max-width:900px;height:auto}</style>
<pre id="status">RENDERING</pre><div id="out"></div>
<script type="module">
import mermaid from "%s";
const blocks = %s;
mermaid.initialize({ startOnLoad: false, securityLevel: "strict" });
const out = document.getElementById("out"), errs = [];
for (const [i, b] of blocks.entries()) {
  const sec = out.appendChild(document.createElement("section"));
  try {
    const { svg } = await mermaid.render("m" + i, b.code);
    sec.innerHTML = "<h3>" + b.src + "</h3>" + svg;
  } catch (e) {
    errs.push(b.src + ": " + String(e.message || e).split("\\n").slice(0, 3).join(" | "));
    sec.innerHTML = "<h3>" + b.src + "</h3><pre class=err></pre>";
    sec.querySelector("pre").textContent = String(e.message || e);
  }
}
document.getElementById("status").textContent =
  errs.length ? "ERRORS\\n" + errs.join("\\n") : "OK " + blocks.length;
</script>""" % (sys.argv[2], json.dumps(blocks))
pathlib.Path(sys.argv[1]).write_text(html, encoding="utf-8")
print(len(blocks))
PY
)

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
page="file://$PWD/$out/mermaid.html"
flags=(--headless=new --hide-scrollbars ${CI:+--no-sandbox} --user-data-dir="$tmp/p" --allow-file-access-from-files --virtual-time-budget=15000)
status=$("$chrome" "${flags[@]}" --dump-dom "$page" 2>/dev/null \
  | python3 -c 'import html,re,sys; m=re.search(r"<pre id=\"status\">(.*?)</pre>", sys.stdin.read(), re.S); print(html.unescape(m.group(1)) if m else "NO RESULT")')
"$chrome" "${flags[@]}" --window-size=1000,$((count * 420 + 200)) --screenshot="$out/mermaid.png" "$page" >/dev/null 2>&1 || true

echo "diagrams: $count   gallery: $out/mermaid.png"
case "$status" in
  OK*) echo "mermaid: all render" ;;
  *) echo "$status" >&2; exit 1 ;;
esac
