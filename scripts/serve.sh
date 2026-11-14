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
import socketserver
import sys
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

class Server(ThreadingHTTPServer):
    # The page loads every level picture and sound at once; with Python's
    # default backlog of 5, macOS resets some of those connections and a
    # picture fails to load now and then.
    request_queue_size = 64

    def server_bind(self):
        # HTTPServer.server_bind calls socket.getfqdn(), a reverse DNS lookup
        # that can stall for ~35 s on macOS (seen with Homebrew Python 3.14).
        socketserver.TCPServer.server_bind(self)
        self.server_name, self.server_port = self.server_address[:2]

class NoCache(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, *args):
        pass  # keep the terminal quiet

Server(("127.0.0.1", int(sys.argv[1])), NoCache).serve_forever()
PY
