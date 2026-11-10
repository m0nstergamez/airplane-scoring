#!/usr/bin/env bash
# Serve web/ on http://localhost:${PORT:-8080} with caching disabled, so a
# browser refresh always loads the freshly built .wasm.
# Usage: scripts/serve.sh            (port 8080)
#        PORT=9000 scripts/serve.sh
set -euo pipefail
cd "$(dirname "$0")/../web"
PORT="${PORT:-8080}"
if [ ! -f airplane-scoring.wasm ]; then
  echo "web/airplane-scoring.wasm missing - run scripts/build-web.sh first." >&2
  exit 1
fi
echo "Serving on http://localhost:$PORT  (Ctrl+C to stop)"
exec python3 - "$PORT" <<'PY'
import sys
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

class NoCache(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, *args):
        pass  # keep the terminal quiet

ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), NoCache).serve_forever()
PY
