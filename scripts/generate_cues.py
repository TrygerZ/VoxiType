"""Generate the two-stage recording sound cues for VoxiType.

Produces ``src-tauri/assets/sound/press.wav`` and ``ready.wav``:
48 kHz, 16-bit, mono PCM WAV files compatible with the custom parser in
``src-tauri/src/sound.rs`` (canonical RIFF/WAVE, PCM format 1, single data
chunk, even byte count).

Run from the repository root:

    python scripts/generate_cues.py

Design
------
Both cues share one woodblock-inspired modal timbre so they read as a
single instrument family alongside the existing stop cue (110 ms,
~333 Hz fundamental, peak -5.3 dBFS). The timbre is a sum of damped
sinusoidal partials at inharmonic ratios (1.0, ~2.76, ~5.40 relative to
f0), with upper partials decaying faster, plus a short low-passed noise
transient that provides the percussive "tock". The noise uses a fixed
seed so output is deterministic.

Press plays the moment the hotkey registers (deep f0, moderate level).
Ready plays once the microphone stream is live: a perfect fifth above
Press (x1.5, consonant upward interval) at a slightly lower peak, since
it may bleed into the captured audio. Each cue opens with a ~1.5 ms
raised-cosine attack (click-free) and closes with a smooth fade, so the
first and last samples are exactly zero and there is no DC offset.
"""

import math
import random
import struct
import wave

# --- Shared synthesis parameters ---------------------------------------------

SAMPLE_RATE = 48_000

# Inharmonic partial ratios relative to f0 (woodblock-like modal series).
PARTIAL_RATIOS = (1.0, 2.76, 5.40)
PARTIAL_AMPS = (1.0, 0.35, 0.12)

# Base decay time constant (ms) for the fundamental; upper partials decay
# progressively faster, which keeps the tail clean and "tocky".
BASE_DECAY_MS = 26.0
PARTIAL_DECAY_SCALE = (1.0, 0.45, 0.22)

# Raised-cosine attack length (ms) to avoid clicks.
ATTACK_MS = 1.5
# Cosine fade-out at the tail (ms) so the final samples land on zero.
FADE_OUT_MS = 12.0

# Percussive transient: burst of low-passed noise at the very start.
TRANSIENT_MS = 1.8
TRANSIENT_AMP = 0.55
TRANSIENT_LOWPASS = 0.22  # one-pole coefficient, lower = darker knock
NOISE_SEED = 20260918

# --- Per-cue parameters -------------------------------------------------------

PRESS = {
    "path": "src-tauri/assets/sound/press.wav",
    "f0": 330.0,          # deep, matches stop cue family (~333 Hz)
    "duration_ms": 120.0,
    "peak": 10.0 ** (-9.0 / 20.0),   # -9 dBFS
}

READY = {
    "path": "src-tauri/assets/sound/ready.wav",
    "f0": 330.0 * 1.5,    # perfect fifth above Press (consonant lift)
    "duration_ms": 110.0,  # slightly shorter tail, cues feel "answered"
    "peak": 10.0 ** (-11.0 / 20.0),  # -11 dBFS, may bleed into the mic
}


def synthesize(f0, duration_ms, peak):
    """Render one woodblock cue and return a list of float samples in [-1, 1]."""
    n_samples = int(round(SAMPLE_RATE * duration_ms / 1000.0))
    attack_n = max(1, int(round(SAMPLE_RATE * ATTACK_MS / 1000.0)))
    fade_n = max(1, int(round(SAMPLE_RATE * FADE_OUT_MS / 1000.0)))
    transient_n = int(round(SAMPLE_RATE * TRANSIENT_MS / 1000.0))

    rng = random.Random(NOISE_SEED)
    noise = [rng.uniform(-1.0, 1.0) for _ in range(transient_n)]
    # One-pole low-pass so the transient reads as a knock, not a hiss.
    lp = 0.0
    for i, v in enumerate(noise):
        lp += TRANSIENT_LOWPASS * (v - lp)
        noise[i] = lp

    out = []
    for n in range(n_samples):
        t_ms = (n / SAMPLE_RATE) * 1000.0

        env_attack = 1.0
        if n < attack_n:
            env_attack = 0.5 - 0.5 * math.cos(math.pi * n / attack_n)

        env_fade = 1.0
        if n >= n_samples - fade_n:
            k = (n_samples - n) / fade_n
            env_fade = 0.5 - 0.5 * math.cos(math.pi * k)

        value = 0.0
        for ratio, amp, dec_scale in zip(PARTIAL_RATIOS, PARTIAL_AMPS, PARTIAL_DECAY_SCALE):
            tau_ms = BASE_DECAY_MS * dec_scale
            env = math.exp(-t_ms / tau_ms)
            value += amp * env * math.sin(2.0 * math.pi * f0 * ratio * n / SAMPLE_RATE)

        if n < transient_n:
            value += TRANSIENT_AMP * noise[n] * (1.0 - n / transient_n)

        out.append(value * env_attack * env_fade)

    # Normalize to the requested peak; then remove any residual DC so the
    # cue neither clips nor carries an offset.
    max_abs = max(abs(v) for v in out) or 1.0
    gain = peak / max_abs
    out = [v * gain for v in out]
    dc = sum(out) / len(out)
    out = [v - dc for v in out]
    out[0] = 0.0
    out[-1] = 0.0
    return out


def write_wav(path, samples):
    frames = struct.pack("<%dh" % len(samples), *[int(round(max(-1.0, min(1.0, v)) * 32767.0)) for v in samples])
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        w.writeframes(frames)


def report(name, path, samples):
    peak = max(max(samples), -min(samples))
    print(
        f"{name}: {path}  {len(samples)} samples  "
        f"{len(samples) / SAMPLE_RATE * 1000.0:.1f} ms  "
        f"peak {20.0 * math.log10(peak):.1f} dBFS  "
        f"first={samples[0]} last={samples[-1]}"
    )


def main():
    for name, cfg in (("press", PRESS), ("ready", READY)):
        samples = synthesize(cfg["f0"], cfg["duration_ms"], cfg["peak"])
        write_wav(cfg["path"], samples)
        report(name, cfg["path"], samples)


if __name__ == "__main__":
    main()
