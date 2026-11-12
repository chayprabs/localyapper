#!/usr/bin/env python3
"""Build the LibriSpeech test-clean WAV fixtures in src-tauri/tests/fixtures.

Usage: make-fixtures.py <path/to/LibriSpeech/test-clean> <out_dir>

Needs numpy, scipy and soundfile. The selection is deterministic (sorted
speaker and utterance ids), so re-running gives the same fixtures.
LibriSpeech is CC-BY-4.0 (Panayotov et al., 2015).
"""
import glob
import os
import sys

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

SRC, OUT = sys.argv[1], sys.argv[2]
RATE = 16000


def utterances():
    """Yield (speaker, utt_id, samples, text) in sorted order."""
    for spk in sorted(os.listdir(SRC), key=int):
        trans = {}
        for t in sorted(glob.glob(os.path.join(SRC, spk, "*", "*.trans.txt"))):
            for line in open(t, encoding="utf-8"):
                uid, text = line.strip().split(" ", 1)
                trans[uid] = text
        for uid in sorted(trans):
            chap = uid.split("-")[1]
            audio, rate = sf.read(os.path.join(SRC, spk, chap, uid + ".flac"), dtype="float32")
            assert rate == RATE
            yield spk, uid, audio, trans[uid]


def write(name, audio, rate, text, subtype="PCM_16"):
    sf.write(os.path.join(OUT, name + ".wav"), audio, rate, subtype=subtype)
    with open(os.path.join(OUT, name + ".txt"), "w", encoding="utf-8") as f:
        f.write(text + "\n")
    print(f"{name}: {len(audio) / rate:.1f} s, {rate} Hz, {subtype}")


os.makedirs(OUT, exist_ok=True)
by_speaker = {}
for spk, uid, audio, text in utterances():
    by_speaker.setdefault(spk, []).append((uid, audio, text))

speakers = sorted(by_speaker, key=int)
single = []  # one 5 to 12 s clip per speaker
for spk in speakers:
    for uid, audio, text in by_speaker[spk]:
        if 5.0 <= len(audio) / RATE <= 12.0:
            single.append((spk, uid, audio, text))
            break

# 01-06: 16 kHz mono 16-bit, six speakers.
for i in range(6):
    _, _, audio, text = single[i]
    write(f"clip{i + 1:02d}_16k", audio, RATE, text)

# 07: 44.1 kHz stereo 16-bit.
_, _, audio, text = single[6]
up = resample_poly(audio, 441, 160).astype(np.float32)
write("clip07_44k1_stereo", np.stack([up, up * 0.9], axis=1), 44100, text)

# 08: 48 kHz mono 32-bit float.
_, _, audio, text = single[7]
write("clip08_48k_float", resample_poly(audio, 3, 1).astype(np.float32), 48000, text, "FLOAT")

# 09: two utterances with a 3 s pause between them.
a = by_speaker[speakers[8]][1]
b = by_speaker[speakers[9]][1]
gap = np.zeros(3 * RATE, dtype=np.float32)
write("clip09_pause", np.concatenate([a[1], gap, b[1]]), RATE, a[2] + " " + b[2])

# 10: about 90 s of one speaker's consecutive utterances, 0.3 s apart.
parts, texts, total = [], [], 0.0
for uid, audio, text in by_speaker[speakers[10]]:
    parts += [audio, np.zeros(int(0.3 * RATE), dtype=np.float32)]
    texts.append(text)
    total += len(audio) / RATE + 0.3
    if total >= 90.0:
        break
write("clip10_long", np.concatenate(parts), RATE, " ".join(texts))
