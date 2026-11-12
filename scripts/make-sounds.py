#!/usr/bin/env python3
"""Synthesise the game's sound effects into web/sounds/*.wav.

The sounds are made from maths (noise, pulses, filters), not recordings,
so there is no licence to track. The output is deterministic: running
this again gives the same files. Only needed when you change a sound;
the .wav files are committed.

Usage: scripts/make-sounds.py            (Python 3, standard library only)

Format: mono, 16 bit, 22050 Hz WAV. WAV is the one format that every
browser (Web Audio) and the native build (quad-snd) can both decode.
"""
import math
import pathlib
import random
import struct
import wave

RATE = 22050
OUT = pathlib.Path(__file__).resolve().parent.parent / "web" / "sounds"


def noise(rng, n):
    return [rng.uniform(-1.0, 1.0) for _ in range(n)]


def lowpass(xs, hz, state=0.0):
    """One-pole low-pass. `hz` is a number or one value per sample."""
    out, y = [], state
    for i, x in enumerate(xs):
        f = hz[i] if isinstance(hz, list) else hz
        y += (1.0 - math.exp(-2.0 * math.pi * f / RATE)) * (x - y)
        out.append(y)
    return out


def highpass(xs, hz):
    return [x - l for x, l in zip(xs, lowpass(xs, hz))]


def looped(make, seconds):
    """Run a periodic generator for two loop lengths and keep the second:
    by then every filter has settled, so end and start join seamlessly."""
    n = int(seconds * RATE)
    return make(n)[n:]


def normalise(xs, peak):
    top = max(abs(x) for x in xs)
    return [x * peak / top for x in xs]


def write(name, xs, peak):
    OUT.mkdir(parents=True, exist_ok=True)
    data = b"".join(struct.pack("<h", round(x * 32767)) for x in normalise(xs, peak))
    with wave.open(str(OUT / name), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(RATE)
        f.writeframes(data)
    print(f"{name}: {len(xs) / RATE:.2f} s, {len(data) // 1024} KB")


def engine(seed, pulses_per_second, cutoff, hiss):
    """Propeller engine: a train of cylinder pulses with a repeating
    pattern (uneven firing gives the putter), through a low-pass, plus
    some prop wash. One second, loops seamlessly."""
    rng = random.Random(seed)
    pattern = [1.0, 0.62, 0.85, 0.5]

    def make(n):
        wash = noise(rng, n)
        xs = [0.0] * (2 * n)
        for k in range(2 * pulses_per_second):
            at = round(k * RATE / pulses_per_second)
            xs[at] += pattern[k % len(pattern)] * 40.0
        xs = lowpass(lowpass(xs, cutoff), cutoff)
        body = highpass(xs, 60.0)
        air = lowpass(highpass(wash + wash, 300.0), cutoff * 3.0)
        # The wash pulses with the blades.
        return [
            b + hiss * a * (0.6 + 0.4 * math.sin(2 * math.pi * pulses_per_second / 2 * i / RATE))
            for i, (b, a) in enumerate(zip(body, air))
        ]

    return looped(make, 1.0)


def wind(seed, seconds):
    """Howling wind: noise through a low-pass whose cutoff wanders."""
    rng = random.Random(seed)

    def make(n):
        src = noise(rng, n)
        cycle = lambda i, k: math.sin(2 * math.pi * k * i / n)  # k waves per loop
        hz = [520.0 + 260.0 * cycle(i, 1) + 140.0 * cycle(i, 3) for i in range(2 * n)]
        xs = lowpass(lowpass(src + src, hz), hz)
        xs = highpass(xs, 120.0)
        return [x * (0.75 + 0.25 * cycle(i, 2)) for i, x in enumerate(xs)]

    return looped(make, seconds)


def rain(seed, seconds):
    """Rain: bright hiss plus many small drops."""
    rng = random.Random(seed)

    def make(n):
        src = noise(rng, n)
        hiss = lowpass(highpass(src + src, 1800.0), 7000.0)
        drops = [0.0] * n
        for _ in range(int(140 * seconds)):
            at, loud, length = rng.randrange(n), rng.uniform(0.3, 1.0), rng.randint(40, 160)
            for j in range(length):
                drops[(at + j) % n] += loud * rng.uniform(-1, 1) * math.exp(-5.0 * j / length)
        drops = lowpass(highpass(drops + drops, 900.0), 5000.0)
        return [h + 0.9 * d for h, d in zip(hiss, drops)]

    return looped(make, seconds)


def thunder(seed, seconds):
    """Thunder: a sharp crack, then rumbles that roll away."""
    rng = random.Random(seed)
    n = int(seconds * RATE)
    crack = lowpass(noise(rng, n), 2500.0)
    low = lowpass(lowpass(noise(rng, n), 110.0), 110.0)
    mid = lowpass(lowpass(noise(rng, n), 420.0), 420.0)  # so phone speakers carry it
    bumps = [(0.0, 1.0)] + [(rng.uniform(0.15, 1.9), rng.uniform(0.35, 0.8)) for _ in range(5)]
    out = []
    for i in range(n):
        t = i / RATE
        roll = sum(a * math.exp(-(t - at) / 0.7) for at, a in bumps if t >= at)
        out.append(
            1.4 * crack[i] * math.exp(-t / 0.06)
            + (9.0 * low[i] + 2.2 * mid[i]) * roll * min(1.0, (seconds - t) / 0.8)
        )
    return out


def crash(seed, seconds):
    """Crash: impact noise, a low thud, ringing metal, then falling bits."""
    rng = random.Random(seed)
    n = int(seconds * RATE)
    bang = lowpass(noise(rng, n), 3200.0)
    rings = [(523.0, 0.22), (811.0, 0.16), (1247.0, 0.2), (1873.0, 0.1), (2690.0, 0.07)]
    hits = [(0.0, 1.0), (0.17, 0.45), (0.36, 0.3), (0.52, 0.15)]
    out = []
    for i in range(n):
        t = i / RATE
        x = 0.0
        for at, loud in hits:
            if t < at:
                continue
            d = t - at
            x += loud * 1.3 * bang[i] * math.exp(-d / 0.07)
            # thud: a sine falling from 150 to 50 Hz
            x += loud * 0.9 * math.sin(2 * math.pi * (50.0 * d + 15.0 * (1 - math.exp(-d / 0.15)))) * math.exp(-d / 0.16)
            x += loud * sum(0.16 * math.sin(2 * math.pi * hz * d) * math.exp(-d / tau) for hz, tau in rings)
        out.append(x * min(1.0, (seconds - t) / 0.1))
    return out


def beach(seed, seconds):
    """Beach (level 2): waves rolling onto the sand, two per loop, the
    second one smaller. Each wave swells slowly, breaks bright and washes
    out; between them the sea whispers. The gulls are their own sound
    (gull-call.wav): each one calls when it appears on screen."""
    rng = random.Random(seed)
    waves = [(0.0, 1.0), (seconds / 2, 0.7)]  # (start in s, strength)

    def swell(t):
        rise, wash = 1.8, 0.9
        return (t / rise) ** 2 if t < rise else math.exp(-(t - rise) / wash)

    def make(n):
        env = []
        for i in range(2 * n):
            env.append(0.12 + sum(a * swell(((i - round(at * RATE)) % n) / RATE) for at, a in waves))
        hz = [250.0 + 1400.0 * min(e, 1.0) for e in env]  # brighter when breaking
        src = noise(rng, n)
        xs = highpass(lowpass(lowpass(src + src, hz), hz), 70.0)
        return [x * e for x, e in zip(xs, env)]

    return looped(make, seconds)


def gull(pitch, length):
    """One "kee-ow": up, then down, with a few overtones."""
    out, phase = [], 0.0
    for i in range(int(length * RATE)):
        t = i / length / RATE  # 0..1 through the call
        hz = pitch * (1.0 + 0.35 * math.sin(math.pi * t) - 0.25 * t)
        phase += 2 * math.pi * hz / RATE
        tone = math.sin(phase) + 0.5 * math.sin(2 * phase) + 0.25 * math.sin(3 * phase)
        out.append(tone * math.sin(math.pi * t) ** 0.6)
    return out


def gull_call(seconds):
    """A gull calling as it appears: three "kee-ow"s, the same calls that
    the first version of the beach loop had."""
    out = [0.0] * int(seconds * RATE)
    for at, pitch, length in [(0.0, 1250.0, 0.30), (0.4, 1300.0, 0.26), (0.75, 1200.0, 0.34)]:
        start = round(at * RATE)
        for j, x in enumerate(gull(pitch, length)):
            out[start + j] += x
    return out


def new_level(seconds):
    """New level: a short rising tune (C, E, G, high C), bright and happy,
    the last note ringing out."""
    n = int(seconds * RATE)
    notes = [(523.25, 0.0, 0.18), (659.25, 0.1, 0.18), (783.99, 0.2, 0.18), (1046.5, 0.3, 0.55)]
    out = [0.0] * n
    for hz, at, ring in notes:
        for i in range(round(at * RATE), n):
            t = i / RATE - at
            p = 2 * math.pi * hz * t
            tone = math.sin(p) + 0.3 * math.sin(2 * p) + 0.15 * math.sin(3 * p)
            out[i] += tone * min(1.0, t / 0.005) * math.exp(-t / ring)
    return [x * min(1.0, (seconds - i / RATE) / 0.1) for i, x in enumerate(out)]


if __name__ == "__main__":
    write("engine-idle.wav", engine(1, pulses_per_second=52, cutoff=380.0, hiss=0.25), 0.8)
    write("engine-power.wav", engine(2, pulses_per_second=92, cutoff=800.0, hiss=0.5), 0.8)
    write("wind.wav", wind(3, 4.0), 0.8)
    write("rain.wav", rain(4, 3.0), 0.55)
    write("thunder.wav", thunder(5, 3.5), 0.9)
    write("crash.wav", crash(6, 1.2), 0.9)
    write("beach.wav", beach(7, 8.0), 0.8)
    write("gull-call.wav", gull_call(1.15), 0.8)
    write("new-level.wav", new_level(1.3), 0.8)
