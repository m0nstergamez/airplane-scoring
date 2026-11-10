#!/usr/bin/env python3
"""Make web/qr.svg: the QR code on the desktop start screen that opens the
game on a phone. Run it again only if the address changes; commit the SVG.

    scripts/make-qr.py [url]

Standard library only, like make-sounds.py. A small QR encoder: byte mode,
error correction level M (survives a bit of glare on a laptop screen),
versions 1 to 6 (up to 106 bytes), the mask with the lowest penalty.
Follows ISO/IEC 18004; the structure follows Nayuki's reference
implementation (https://www.nayuki.io/page/qr-code-generator-library).
"""

import pathlib
import sys

URL = "https://m0nstergamez.github.io/airplane-scoring/"
OUT = pathlib.Path(__file__).resolve().parent.parent / "web" / "qr.svg"

# Level M per version: (error correction codewords per block, number of
# blocks, data codewords per block). All blocks are the same size up to v6.
LEVEL_M = {1: (10, 1, 16), 2: (16, 1, 28), 3: (26, 1, 44), 4: (18, 2, 32), 5: (24, 2, 43), 6: (16, 4, 27)}
ALIGNMENT = {1: [], 2: [6, 18], 3: [6, 22], 4: [6, 26], 5: [6, 30], 6: [6, 34]}
REMAINDER_BITS = {1: 0, 2: 7, 3: 7, 4: 7, 5: 7, 6: 7}
FORMAT_M = 0  # level M in the format bits (L=1, M=0, Q=3, H=2)


# --- Reed-Solomon over GF(256), polynomial 0x11D -------------------------

def gf_mul(x, y):
    z = 0
    for i in range(7, -1, -1):
        z = (z << 1) ^ ((z >> 7) * 0x11D)
        z ^= ((y >> i) & 1) * x
    return z


def rs_divisor(degree):
    result = [0] * (degree - 1) + [1]
    root = 1
    for _ in range(degree):
        for j in range(degree):
            result[j] = gf_mul(result[j], root)
            if j + 1 < degree:
                result[j] ^= result[j + 1]
        root = gf_mul(root, 0x02)
    return result


def rs_remainder(data, divisor):
    result = [0] * len(divisor)
    for b in data:
        factor = b ^ result.pop(0)
        result.append(0)
        for i, coef in enumerate(divisor):
            result[i] ^= gf_mul(coef, factor)
    return result


# --- Codewords -------------------------------------------------------------

def codewords(text):
    data = text.encode("utf-8")
    for version, (ec_len, blocks, block_len) in LEVEL_M.items():
        capacity = blocks * block_len
        if 4 + 8 + 8 * len(data) <= capacity * 8:
            break
    else:
        sys.exit(f"too long for version 6-M: {len(data)} bytes")

    bits = [0, 1, 0, 0]  # byte mode
    bits += [(len(data) >> i) & 1 for i in range(7, -1, -1)]
    for b in data:
        bits += [(b >> i) & 1 for i in range(7, -1, -1)]
    bits += [0] * min(4, capacity * 8 - len(bits))  # terminator
    bits += [0] * (-len(bits) % 8)
    words = [int("".join(map(str, bits[i:i + 8])), 2) for i in range(0, len(bits), 8)]
    pad = [0xEC, 0x11]
    while len(words) < capacity:
        words.append(pad[(len(words) - len(bits) // 8) % 2])

    divisor = rs_divisor(ec_len)
    data_blocks = [words[i * block_len:(i + 1) * block_len] for i in range(blocks)]
    ec_blocks = [rs_remainder(b, divisor) for b in data_blocks]
    out = [b[i] for i in range(block_len) for b in data_blocks]
    out += [b[i] for i in range(ec_len) for b in ec_blocks]
    return version, out


# --- Matrix ----------------------------------------------------------------

class Matrix:
    def __init__(self, version):
        self.version = version
        self.size = 17 + 4 * version
        self.dark = [[False] * self.size for _ in range(self.size)]
        self.function = [[False] * self.size for _ in range(self.size)]

    def set_function(self, x, y, dark):
        self.dark[y][x] = dark
        self.function[y][x] = True

    def draw_function_patterns(self):
        n = self.size
        for i in range(n):  # timing patterns
            self.set_function(6, i, i % 2 == 0)
            self.set_function(i, 6, i % 2 == 0)
        for cx, cy in ((3, 3), (n - 4, 3), (3, n - 4)):  # finders + separators
            for dy in range(-4, 5):
                for dx in range(-4, 5):
                    x, y = cx + dx, cy + dy
                    if 0 <= x < n and 0 <= y < n:
                        dist = max(abs(dx), abs(dy))
                        self.set_function(x, y, dist not in (2, 4))
        pos = ALIGNMENT[self.version]
        for i, ax in enumerate(pos):
            for j, ay in enumerate(pos):
                corner = (i == 0 and j == 0) or (i == 0 and j == len(pos) - 1) or (i == len(pos) - 1 and j == 0)
                if corner:
                    continue
                for dy in range(-2, 3):
                    for dx in range(-2, 3):
                        self.set_function(ax + dx, ay + dy, max(abs(dx), abs(dy)) != 1)
        self.draw_format(0)  # reserve the format areas; real bits come later

    def draw_format(self, mask):
        data = FORMAT_M << 3 | mask
        rem = data
        for _ in range(10):
            rem = (rem << 1) ^ ((rem >> 9) * 0x537)
        bits = (data << 10 | rem) ^ 0x5412
        bit = lambda i: (bits >> i) & 1 == 1
        n = self.size
        for i in range(6):
            self.set_function(8, i, bit(i))
        self.set_function(8, 7, bit(6))
        self.set_function(8, 8, bit(7))
        self.set_function(7, 8, bit(8))
        for i in range(9, 15):
            self.set_function(14 - i, 8, bit(i))
        for i in range(8):
            self.set_function(n - 1 - i, 8, bit(i))
        for i in range(8, 15):
            self.set_function(8, n - 15 + i, bit(i))
        self.set_function(8, n - 8, True)  # the dark module

    def place_data(self, words):
        n = self.size
        i = 0
        total = len(words) * 8
        right = n - 1
        while right >= 1:
            if right == 6:
                right = 5
            for vert in range(n):
                for j in range(2):
                    x = right - j
                    upward = ((right + 1) & 2) == 0
                    y = n - 1 - vert if upward else vert
                    if not self.function[y][x] and i < total:
                        self.dark[y][x] = (words[i >> 3] >> (7 - (i & 7))) & 1 == 1
                        i += 1
                    # Remainder bits stay light (False).
            right -= 2

    def apply_mask(self, mask):
        rule = [
            lambda x, y: (x + y) % 2 == 0,
            lambda x, y: y % 2 == 0,
            lambda x, y: x % 3 == 0,
            lambda x, y: (x + y) % 3 == 0,
            lambda x, y: (x // 3 + y // 2) % 2 == 0,
            lambda x, y: x * y % 2 + x * y % 3 == 0,
            lambda x, y: (x * y % 2 + x * y % 3) % 2 == 0,
            lambda x, y: ((x + y) % 2 + x * y % 3) % 2 == 0,
        ][mask]
        for y in range(self.size):
            for x in range(self.size):
                if not self.function[y][x] and rule(x, y):
                    self.dark[y][x] = not self.dark[y][x]

    def penalty(self):
        n, m = self.size, self.dark
        score = 0
        lines = [m[y] for y in range(n)] + [[m[y][x] for y in range(n)] for x in range(n)]
        finder = [True, False, True, True, True, False, True]
        for line in lines:
            run, prev = 0, None
            for c in line + [None]:
                if c == prev:
                    run += 1
                else:
                    if run >= 5:
                        score += 3 + run - 5
                    run, prev = 1, c
            padded = [False] * 4 + line + [False] * 4
            for i in range(len(padded) - 10):
                w = padded[i:i + 11]
                if w[:7] == finder and not any(w[7:]) or w[4:] == finder and not any(w[:4]):
                    score += 40
        for y in range(n - 1):
            for x in range(n - 1):
                if m[y][x] == m[y][x + 1] == m[y + 1][x] == m[y + 1][x + 1]:
                    score += 3
        dark = sum(map(sum, m))
        score += 10 * (abs(dark * 20 - n * n * 10) // (n * n))
        return score


def encode(text):
    version, words = codewords(text)
    best = None
    for mask in range(8):
        qr = Matrix(version)
        qr.draw_function_patterns()
        qr.place_data(words)
        qr.apply_mask(mask)
        qr.draw_format(mask)
        p = qr.penalty()
        if best is None or p < best[0]:
            best = (p, qr)
    return best[1]


def svg(qr):
    n = qr.size
    path = "".join(f"M{x} {y}h1v1h-1z" for y in range(n) for x in range(n) if qr.dark[y][x])
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" '
        f'shape-rendering="crispEdges"><path fill="#000" d="{path}"/></svg>\n'
    )


if __name__ == "__main__":
    url = sys.argv[1] if len(sys.argv) > 1 else URL
    qr = encode(url)
    OUT.write_text(svg(qr), encoding="utf-8")
    print(f"{OUT.relative_to(OUT.parent.parent)} -> {url} (version {qr.version}, {qr.size}x{qr.size})")
