#!/usr/bin/env python3
"""Real-time headless screenshot via the Chrome DevTools Protocol.

Used by scripts/shot.sh. Chrome's --virtual-time-budget does not drive
requestAnimationFrame, so a game loop barely runs before the shot (about 17
ticks whatever the budget). This script instead loads the page, waits real
wall-clock time, then captures the screen, so `ms` means what it says.

Standard library only (a minimal WebSocket client is included), so it needs
nothing beyond python3 and Chrome.

Usage: cdp_shot.py CHROME URL OUT.png WIDTH,HEIGHT MS PROFILE_DIR
Prints every console message, uncaught exception and log entry, one per
line, to stdout; shot.sh filters known noise. Exit code 0 unless Chrome or
the protocol fails.
"""

import base64
import json
import os
import socket
import struct
import subprocess
import sys
import time
import urllib.request


class WebSocket:
    """Just enough RFC 6455 for one local DevTools connection."""

    def __init__(self, url):
        assert url.startswith("ws://"), url
        hostport, _, path = url[len("ws://"):].partition("/")
        host, _, port = hostport.partition(":")
        self.sock = socket.create_connection((host, int(port)), timeout=30)
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall(
            (
                "GET /{} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\n"
                "Connection: Upgrade\r\nSec-WebSocket-Key: {}\r\n"
                "Sec-WebSocket-Version: 13\r\n\r\n"
            ).format(path, hostport, key).encode()
        )
        head = b""
        while b"\r\n\r\n" not in head:
            chunk = self.sock.recv(1)
            if not chunk:
                raise ConnectionError("DevTools closed during handshake")
            head += chunk
        if b" 101 " not in head.split(b"\r\n", 1)[0]:
            raise ConnectionError("DevTools handshake failed: " + head.decode(errors="replace"))

    def _exact(self, n):
        buf = b""
        while len(buf) < n:
            chunk = self.sock.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("DevTools connection closed")
            buf += chunk
        return buf

    def send(self, text):
        data = text.encode()
        n = len(data)
        if n < 126:
            header = struct.pack("!BB", 0x81, 0x80 | n)
        elif n < 1 << 16:
            header = struct.pack("!BBH", 0x81, 0x80 | 126, n)
        else:
            header = struct.pack("!BBQ", 0x81, 0x80 | 127, n)
        mask = os.urandom(4)
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
        self.sock.sendall(header + mask + masked)

    def recv(self):
        message = b""
        while True:
            b0, b1 = self._exact(2)
            n = b1 & 0x7F
            if n == 126:
                n = struct.unpack("!H", self._exact(2))[0]
            elif n == 127:
                n = struct.unpack("!Q", self._exact(8))[0]
            payload = self._exact(n)
            opcode = b0 & 0x0F
            if opcode == 0x8:
                raise ConnectionError("DevTools closed the connection")
            if opcode in (0x9, 0xA):  # ping/pong: Chrome does not need replies here
                continue
            message += payload
            if b0 & 0x80:  # FIN
                return message.decode()


class DevTools:
    def __init__(self, ws_url):
        self.ws = WebSocket(ws_url)
        self.next_id = 0
        self.lines = []  # console output, in order
        self.events = []  # method names seen

    def _handle(self, msg):
        method = msg.get("method")
        if not method:
            return
        self.events.append(method)
        p = msg.get("params", {})
        if method == "Runtime.consoleAPICalled":
            parts = []
            for a in p.get("args", []):
                parts.append(str(a.get("value", a.get("description", a.get("type", "")))))
            self.lines.append("console.{}: {}".format(p.get("type"), " ".join(parts)))
        elif method == "Runtime.exceptionThrown":
            d = p.get("exceptionDetails", {})
            exc = d.get("exception", {})
            self.lines.append("Uncaught: {}".format(exc.get("description") or d.get("text")))
        elif method == "Log.entryAdded":
            e = p.get("entry", {})
            self.lines.append("log.{}: {} {}".format(e.get("level"), e.get("text"), e.get("url", "")).rstrip())

    def call(self, method, **params):
        self.next_id += 1
        my_id = self.next_id
        self.ws.send(json.dumps({"id": my_id, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("id") == my_id:
                if "error" in msg:
                    raise RuntimeError("{}: {}".format(method, msg["error"]))
                return msg.get("result", {})
            self._handle(msg)

    def pump_until(self, deadline, stop_event=None):
        """Read events until `deadline` (time.monotonic) or `stop_event` arrives."""
        while True:
            left = deadline - time.monotonic()
            if left <= 0:
                return False
            self.ws.sock.settimeout(left)
            try:
                msg = json.loads(self.ws.recv())
            except socket.timeout:
                return False
            finally:
                self.ws.sock.settimeout(30)
            self._handle(msg)
            if stop_event and msg.get("method") == stop_event:
                return True


def main():
    chrome, url, out, size, ms, profile = sys.argv[1:7]
    width, height = (int(v) for v in size.split(","))
    proc = subprocess.Popen(
        [
            chrome,
            "--headless=new",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--window-size={},{}".format(width, height),
            "--hide-scrollbars",
            "--user-data-dir=" + profile,
            "--remote-debugging-port=0",
            "--no-first-run",
            "--no-default-browser-check",
            "about:blank",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        port_file = os.path.join(profile, "DevToolsActivePort")
        deadline = time.monotonic() + 20
        while not os.path.exists(port_file) or os.path.getsize(port_file) == 0:
            if time.monotonic() > deadline or proc.poll() is not None:
                raise RuntimeError("Chrome did not start DevTools")
            time.sleep(0.05)
        port = open(port_file).read().split()[0]
        with urllib.request.urlopen("http://127.0.0.1:{}/json/list".format(port)) as r:
            pages = [t for t in json.load(r) if t.get("type") == "page"]
        dt = DevTools(pages[0]["webSocketDebuggerUrl"])
        dt.call("Runtime.enable")
        dt.call("Log.enable")
        dt.call("Page.enable")
        dt.call(
            "Emulation.setDeviceMetricsOverride",
            width=width,
            height=height,
            deviceScaleFactor=1,
            mobile=False,
        )
        dt.call("Page.navigate", url=url)
        if not dt.pump_until(time.monotonic() + 20, "Page.loadEventFired"):
            raise RuntimeError("page did not finish loading within 20 s")
        # Real time from here on: the game loop runs at the browser's frame rate.
        dt.pump_until(time.monotonic() + int(ms) / 1000.0)
        shot = dt.call("Page.captureScreenshot", format="png")
        with open(out, "wb") as f:
            f.write(base64.b64decode(shot["data"]))
        # One line per message (stack traces joined), so shot.sh can filter
        # a known message as a whole.
        for line in dt.lines:
            print(" ⏎ ".join(part.strip() for part in line.splitlines()))
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    try:
        main()
    except Exception as e:  # report cleanly; shot.sh turns this into a failure
        print("cdp_shot.py: {}".format(e), file=sys.stderr)
        sys.exit(1)
