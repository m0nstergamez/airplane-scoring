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


def turbine(seed, whine_hz, roar_hz, blast, seconds=2.0):
    """Turbine (jet) engine: the whine of the spinning compressor (two
    close tones, so it shimmers, with a little overtone), a low hum, and
    the broad roar of the exhaust; `blast` adds the bright hiss of full
    thrust. Every tone has a whole number of cycles in the loop and the
    noise is run twice (see `looped`), so it loops seamlessly."""
    rng = random.Random(seed)
    n = int(seconds * RATE)

    def make(m):
        src = noise(rng, m)
        both = src + src
        roar = highpass(lowpass(lowpass(both, roar_hz), roar_hz), 80.0)
        hiss = lowpass(highpass(both, 2500.0), 7000.0)
        return [r + blast * h for r, h in zip(roar, hiss)]

    air = looped(make, seconds)
    tones = [(whine_hz, 0.10), (whine_hz + 3.0, 0.08), (2 * whine_hz, 0.03), (140.0, 0.12)]
    out = []
    for i in range(n):
        t = i / RATE
        tone = sum(a * math.sin(2 * math.pi * hz * t) for hz, a in tones)
        # The roar breathes a little (once per loop, so it still joins).
        swell = 0.9 + 0.1 * math.sin(2 * math.pi * t / seconds)
        out.append(tone + 0.9 * swell * air[i])
    return out


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


def burner(seed, seconds):
    """Hot-air balloon burner: a soft roar of gas flame that swells in,
    holds with a gentle flutter, and dies away."""
    rng = random.Random(seed)
    n = int(seconds * RATE)
    src = noise(rng, n)
    roar = highpass(lowpass(lowpass(src, 900.0), 900.0), 90.0)
    hiss = lowpass(highpass(src, 2500.0), 6000.0)
    out = []
    for i in range(n):
        t = i / RATE
        env = min(1.0, t / 0.15) * min(1.0, (seconds - t) / 0.35)
        flutter = 0.85 + 0.15 * math.sin(2 * math.pi * 11.0 * t)
        out.append(env * flutter * (roar[i] + 0.25 * hiss[i]))
    return out


def explosion(seed, seconds):
    """Explosion: a sharp blast, a deep boom that drops in pitch, a
    rumbling cloud that rolls away, and crackling bits."""
    rng = random.Random(seed)
    n = int(seconds * RATE)
    blast = lowpass(noise(rng, n), 3000.0)
    rumble = lowpass(lowpass(noise(rng, n), 160.0), 160.0)
    out = []
    phase = 0.0
    for i in range(n):
        t = i / RATE
        hz = 30.0 + 60.0 * math.exp(-t / 0.25)  # boom: 90 Hz down to 30 Hz
        phase += 2 * math.pi * hz / RATE
        x = 1.6 * blast[i] * math.exp(-t / 0.08)
        x += 1.2 * math.sin(phase) * math.exp(-t / 0.5)
        x += 7.0 * rumble[i] * math.exp(-t / 0.9)
        out.append(x)
    # Crackles: short bright ticks scattered over the first second.
    for _ in range(40):
        at, loud = rng.randrange(int(0.1 * RATE), int(1.2 * RATE)), rng.uniform(0.2, 0.6)
        for j in range(120):
            if at + j < n:
                out[at + j] += loud * rng.uniform(-1, 1) * math.exp(-j / 25.0)
    return [x * min(1.0, (seconds - i / RATE) / 0.3) for i, x in enumerate(out)]


def fanfare(seconds):
    """New highscore: a short brassy fanfare, "ta-ta-ta-taaa", ending on a
    bright chord that rings out. Brass-like: a few strong overtones and a
    quick swell at each note's start."""
    n = int(seconds * RATE)
    g, c, e, g2, c2 = 392.0, 523.25, 659.25, 783.99, 1046.5
    notes = [(g, 0.0, 0.12), (c, 0.13, 0.12), (e, 0.26, 0.12), (g2, 0.39, 0.12)]
    chord = [(c, 0.55), (e, 0.55), (g2, 0.55), (c2, 0.55)]
    out = [0.0] * n

    def brass(hz, t):
        p = 2 * math.pi * hz * t
        return math.sin(p) + 0.6 * math.sin(2 * p) + 0.4 * math.sin(3 * p) + 0.2 * math.sin(4 * p)

    for hz, at, length in notes:
        for i in range(round(at * RATE), min(n, round((at + length) * RATE))):
            t = i / RATE - at
            env = min(1.0, t / 0.02) * min(1.0, (length - t) / 0.03)
            out[i] += brass(hz, t) * env
    for hz, at in chord:
        for i in range(round(at * RATE), n):
            t = i / RATE - at
            env = min(1.0, t / 0.03) * math.exp(-t / 0.6)
            out[i] += 0.5 * brass(hz, t) * env
    return [x * min(1.0, (seconds - i / RATE) / 0.15) for i, x in enumerate(out)]


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
    write("engine-idle.wav", turbine(1, whine_hz=1400.0, roar_hz=600.0, blast=0.1), 0.8)
    write("engine-power.wav", turbine(2, whine_hz=1800.0, roar_hz=1300.0, blast=0.35), 0.9)
    write("wind.wav", wind(3, 4.0), 0.8)
    write("rain.wav", rain(4, 3.0), 0.55)
    write("thunder.wav", thunder(5, 3.5), 0.9)
    write("crash.wav", crash(6, 1.2), 0.9)
    write("beach.wav", beach(7, 8.0), 0.8)
    write("gull-call.wav", gull_call(1.15), 0.8)
    write("new-level.wav", new_level(1.3), 0.8)
    write("balloon-burner.wav", burner(8, 1.1), 0.8)
    write("new-highscore.wav", fanfare(1.8), 0.8)
    write("explosion.wav", explosion(9, 2.5), 0.95)
