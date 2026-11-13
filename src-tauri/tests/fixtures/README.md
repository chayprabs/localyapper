# Test fixtures

The `clip*.wav` files and their `clip*.txt` transcripts are built from the
LibriSpeech ASR corpus, test-clean subset, by `scripts/make-fixtures.py`
(run in CI by `.github/workflows/fixtures.yml`).

LibriSpeech: V. Panayotov, G. Chen, D. Povey and S. Khudanpur,
"Librispeech: an ASR corpus based on public domain audio books", ICASSP 2015.
https://www.openslr.org/12. Licence: CC BY 4.0
(https://creativecommons.org/licenses/by/4.0/). The clips were resampled,
converted to stereo or float, and concatenated with silence; no other
changes were made.

| File | Format | Purpose |
|---|---|---|
| `clip01_16k` to `clip06_16k` | 16 kHz mono, 16-bit | Baseline accuracy |
| `clip07_44k1_stereo` | 44.1 kHz stereo, 16-bit | Downmix and resampling |
| `clip08_48k_float` | 48 kHz mono, 32-bit float | Float input and resampling |
| `clip09_pause` | 16 kHz mono, 16-bit | Two utterances with a 3 s pause |
| `clip10_long` | 16 kHz mono, 16-bit | About 90 s of speech |
