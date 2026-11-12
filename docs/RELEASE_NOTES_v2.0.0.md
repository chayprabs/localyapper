# LocalYapper 2.0.0

A rebuild of the dictation pipeline, the overlay and the settings app.
Everything still runs on your computer.

## Faster text

- The speech model starts loading the moment you press the key, and
  each phrase is recognised while you are still talking, so only the
  last phrase is left when you let go.
- No fixed wait after recording: the text is pasted as soon as it is
  ready, and the clipboard is restored afterwards.

## Speech models

- Choose between Parakeet 110M (default), Parakeet 0.6B v3 (25 European
  languages) and Moonshine Tiny (picked automatically on machines with
  less than 6 GB of RAM).
- Downloads resume after an interruption and every file is checked
  against a pinned SHA-256.
- The model is unloaded after an idle period you choose (2 minutes by
  default) and memory is returned to the system.

## Dictation

- Hold the key to talk, or tap it for hands-free; or choose hold-only or
  press-to-toggle.
- The overlay never takes focus and lets clicks through. It shows the
  real microphone level and, if you like, the text as it is recognised.
- Paste method per your apps: Ctrl/Cmd+V, Ctrl+Shift+V for terminals,
  Shift+Insert, typing, or copy only.
- `localyapper --toggle | --start | --stop | --cancel` drives the running
  app; a second launch opens settings.

## Settings app

- New pages: Dictation, Audio (microphone, sound cues and a test
  dictation with timings), Speech, General (launch at login, appearance,
  history retention, About).
- Dark theme.
- History is grouped by day, searchable, exportable and can expire.
- Every setting takes effect immediately and is checked by the app.

## Memory and startup

- The settings window is only created when you open it and is released
  when you close it; starting at login creates no window.
- Measured memory per state for each OS is in `docs/PERFORMANCE.md`.

## Install notes

Builds are not code-signed. See the README for the one-time steps on
Windows (SmartScreen) and macOS (quarantine flag and Accessibility).
