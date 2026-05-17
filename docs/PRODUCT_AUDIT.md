# LocalYapper Product Audit

Date: 2026-05-17
Branch: `main`
Reviewer: codebase-vs-spec reconciliation

This document compares the long-form product spec ("LocalYapper — What This App
Should Be") against the live code. The original audit produced the gap list
below; phases A–F have since been implemented in this repo and are marked
complete. The "Direction reconciliation" section is unchanged because those
items were intentionally superseded and remain out of scope.

---

## 1. Direction reconciliation

The spec was written before the project pivoted to a Parakeet-only product.
Several spec items have been deliberately superseded by the current direction
and are **not** gaps to close. They should be considered **removed from scope**:

| Spec item | Reason it is out of scope | Where the code reflects this |
|---|---|---|
| Models page with three segments (Local + Ollama + BYOK) | Current product is local-only, Parakeet-only. No external services. | `src/components/models/ModelsPage.tsx`, `README.md`, `docs/FINAL_AUDIT.md` |
| Onboarding step 4 "Speech engine setup" with choices | Only one engine exists. The download step *is* the engine setup. | `src/components/wizard/Wizard.tsx` |
| Onboarding step 5 "Training" with 15 paragraphs | Training existed solely to seed the corrections engine, which is gone. | Deleted in commit "Remove obsolete LLM and cleanup UI" |
| Dictionary page with Corrections + Training tabs | Corrections engine is gone. Nothing meaningful to put on the page. | `src/components/dictionary/` was deleted; sidebar has 4 entries |
| Self-improving corrections with confidence decay | Removed deliberately when the LLM and learning surface was cut. | `src-tauri/src/correction/` was deleted; `db/schema.rs` drops the tables |
| Dashboard "connection status for connected speech services" | Local-only app, no remote services to check. | `src/components/dashboard/ModelStatusCard.tsx` only shows local engine state |
| Pipeline correction pass before injection | Pipeline is now `capture -> VAD -> STT -> injection`. | `src-tauri/src/hotkey/manager.rs::run_pipeline_and_inject` |

**Action for these:** the spec, README, and any internal planning docs should
be updated so they no longer promise these features. They are intentionally
gone and will not be re-added unless product direction changes again.

---

## 2. What the current app already does correctly

The vast majority of the spec is implemented. Verified against live code:

### Core dictation experience

- Hold-to-dictate hotkey, configurable, default `F8`
  (`src-tauri/src/hotkey/manager.rs`).
- Overlay appears immediately on press, drives a live waveform during
  recording, transitions to processing on release, shows a transcript preview,
  then auto-dismisses (`src/components/overlay/Overlay.tsx`).
- Text is injected into the previously-focused window using the
  clipboard save -> paste -> restore flow
  (`src-tauri/src/injection/injector.rs`,
  `src-tauri/src/context/detector.rs`).
- Audio stays in RAM. No disk writes during dictation.
- 120-second hard cap with the warning at 105 seconds, both enforced on the
  backend side
  (`MAX_RECORDING_SECONDS`, `RECORDING_WARNING_SECONDS` in
  `hotkey/manager.rs`).
- A 30-second pipeline safety timeout exists.

### Resilience and "do not lose user words"

- The history entry is saved **before** injection is attempted, so if paste
  fails the transcript is still recoverable (`hotkey/manager.rs`).
- On paste failure, the overlay stays visible with a "Paste failed" indicator
  and a copy button so the user can grab the text manually
  (`src/components/overlay/Overlay.tsx`, `error` branch).
- For pipeline failures with no transcript yet, the overlay shows a visible
  `error` state with the backend's error message instead of silently hiding.

### Overlay states

The spec lists 5 states. The code has the same 5 plus 2 helpful extras:

1. `listening` (waveform animates with mic level)
2. `stopping-soon` (red, countdown 15 -> 0)
3. `processing` (spinner)
4. `long-recording` (spinner + minute count for slower transcriptions)
5. `transcribed` (preview + 1.5s sweep bar + auto dismiss)
6. `no-speech` (small "no speech detected" pill)
7. `error` (visible error before transcript exists)

### Main app structure

Sidebar pages (`src/components/settings/Sidebar.tsx`):

- Dashboard (stats + last dictation card with copy/delete)
- History (paginated list, copy/delete per entry, clear-all with confirm)
- Hotkeys (rebind via key-listening mode, reset to defaults, error inline)
- Speech / Models (download progress with bytes/total/speed, file location and
  size, remove button, engine start state)

### Privacy and platform hygiene

- All processing local. No cloud paths, no API keys, no telemetry.
- `rusqlite` bundled; Linux X11 + Wayland branches are present in injection.
- Cross-platform CI (Windows, macOS Intel, macOS Apple Silicon, Linux) green
  per `docs/FINAL_AUDIT.md`.
- Strict TypeScript. No `any`, no `unwrap` in production Rust paths,
  `clippy --all-targets -D warnings` is enforced.

### First-launch onboarding (partial)

- A wizard exists, gated by the `setup_complete` setting in SQLite. After
  completion it never reappears. (`src/components/wizard/Wizard.tsx`,
  `src-tauri/src/db/schema.rs`.)

---

## 3. Real gaps still to close

These are spec requirements that survive the direction reconciliation **and**
are not yet in the code. They are the actual to-do list before declaring
v0.1.0 shippable.

### G1. Wizard does not match the simplified spec

Current order: `Welcome -> Download -> DownloadComplete -> Hotkey -> Ready`.
Recommended order for the simplified Parakeet-only product:

`Welcome -> Microphone -> Hotkey -> Speech files -> Done` (4 or 5 visible
steps).

Specific issues:

- **No microphone permission step.** If the OS denies mic access, the user
  silently ends up with a broken pipeline. Spec calls this out explicitly.
  We have `commands::system::check_permissions` and
  `open_mic_settings`, but the wizard never uses them.
- **No "Step X of N" indicator.** Spec is explicit about this.
- **No back navigation.** User can go forward or skip; they cannot return to
  edit a previous answer. Spec is explicit about this.
- **No granular resume.** Resumability is binary (`setup_complete=true|false`).
  If the user quits at the hotkey step, they restart from welcome on next
  launch. Spec says "resume exactly where they left off." We need a
  `setup_step` setting (or equivalent) persisted across launches.
- **Download is the second step rather than later.** The spec ordering puts
  user-facing decisions (hotkey, mic) before the long download. Reordering
  also means the user sees real progress while making fast decisions.
- **`Skip setup`** silently leaves the model uninstalled with no later cue.
  Either remove the skip option, or surface a clearer "speech files missing"
  state on the Dashboard / Speech page on next launch.

### G2. Hands-free trigger does not match spec

- Code: separate hotkey `Ctrl+F8` (`hotkey_hands_free` setting,
  `manager.rs::handle_hands_free_pressed`).
- Spec: **double-tap of the configured dictation hotkey**.

This is a product decision. Either:

- **Adopt double-tap** (matches spec, removes a reserved hotkey, more elegant).
  Requires implementing a tap-window detector in `hotkey/manager.rs`. Common
  thresholds: ~300ms second tap window, no other key in between.
- **Keep separate hotkey** (matches existing UI, less work). Update the spec
  and the Hotkeys page copy to make this explicit.

Recommendation: adopt double-tap and drop the `hotkey_hands_free` setting.

### G3. Onboarding "summary" step is thin

`ReadyStep.tsx` only shows the chosen hotkey. Spec says step 6 should be
"a summary of the choices the user made." With the simplified product the
choices are: hotkey, microphone status, speech files installed. The summary
should reflect all three.

### G4. Dashboard empty state and model details

- Stat cards show empty placeholders when there is no data, rather than the
  empty state the spec describes ("explain what will appear here once they
  dictate"). Some empty-state copy already exists in `LastDictationCard`
  but not in the cluster of `StatCard` widgets.
- Model status card says "Engine ready" / "Engine not ready" but does not
  surface the model name (`Parakeet 110M`) or file size. Useful for trust.

### G5. Hotkey rebinder does not detect conflicts with reserved combos

The rebinder accepts whatever the OS captures and only fails on duplicate
LocalYapper hotkeys. It does not warn when the user picks something the OS
itself has reserved (e.g. `Ctrl+Alt+Del` style). Lower priority, but worth
noting before release.

### G6. Documentation drift

- `AGENTS.md` and `CLAUDE.md` were deleted from the repo in the recent
  rebase, but the workspace rules still surface their old content (for
  example the rule listing Ollama + BYOK + correction learning under the
  "Voice pipeline data flow" section).
  These should be re-added at the repo root with the **current** simplified
  description so all contributors and agents see the right guidance. The
  cached versions still contain stale claims like "Auto-inject delay 10
  seconds" and a 6-table SQLite layout that no longer exists.
- `README.md` correctly reflects the simplified direction.
- `docs/FINAL_AUDIT.md` already states the direction clearly.
- `docs/PROGRESS.md` is gone (intentional, ignored). No replacement is
  needed.

### G7. Privacy promise visibility

The spec emphasizes the local-only promise repeatedly. The wizard's welcome
screen and the Dashboard could reinforce this once. Currently only the
README mentions it. This is small but cheap and reinforces trust.

### G8. Tray "Pause" reflection in UI

The backend has a `paused` flag (`AppState::paused`), and the tray exposes a
pause action. There is no visible indication in the main app or overlay that
dictation is paused, so a user who toggles pause from the tray could later
press the hotkey, get nothing, and have no idea why. Either:

- Add a "Paused" indicator chip to the main window header, or
- Show a one-time toast / overlay flash when the hotkey is pressed while
  paused.

---

## 4. Phased plan and status

All six phases below have been implemented. Each phase ended in green
lint + typecheck + clippy + tests, and the repository is at parity with the
reconciled spec.

| Phase | Description | Status |
|---|---|---|
| A | Documentation and direction | Done — `AGENTS.md`/`CLAUDE.md` re-tracked at the repo root with current direction |
| B | Onboarding refresh (mic step, indicator, back, summary, granular resume) | Done — wizard now follows Welcome → Microphone → Hotkey → Speech files → Done |
| C | Hands-free unification | Done — double-tap of the record key replaces the separate `Ctrl+F8` hotkey |
| D | Dashboard polish | Done — empty state, model name + size, privacy line, missing-files banner |
| E | Tray pause reflection | Done — `paused-state-changed` event + `Paused` chip in `TitleBar` |
| F | Hotkey reservation warnings | Done — soft-warning under each hotkey row for OS-reserved combos |

### Phase A — Documentation and direction

Smallest, safest first. Establishes the source of truth for the rest.

1. Restore `AGENTS.md` and `CLAUDE.md` at the repo root with the **current**
   speech-only direction (no Ollama, no BYOK, no LLM cleanup, no corrections).
2. Update the long product spec ("What This App Should Be") to reflect the
   reconciled scope from section 1 of this audit, or mark it as historical
   and let `README.md` + this audit be the spec of record.
3. Optional: add a short `docs/SCOPE.md` that lists the decisions in section 1
   so future contributors do not re-litigate them.

### Phase B — Onboarding gaps (G1, G3)

The most user-visible gap. Has to be done before anyone is allowed to install
a fresh build.

1. Add a `setup_step` string setting (`welcome | mic | hotkey | files | done`)
   so the wizard can resume on relaunch (G1 resume).
2. Add a `MicrophoneStep` component that uses `check_permissions` and
   surfaces an "Open mic settings" CTA on denial (G1 mic).
3. Add a `WizardChrome` shell that renders "Step X of N", a Back button, and
   keeps the calm full-screen layout consistent (G1 indicator, G1 back).
4. Reorder the steps to `Welcome -> Mic -> Hotkey -> Speech files -> Done`
   and remove the standalone `DownloadCompleteStep` (it merges into the next
   step or into the summary) (G1 ordering).
5. Expand `ReadyStep` into a real summary screen (G3).
6. Decide what to do about "Skip setup". Recommendation: remove it; users can
   leave the speech files for later from the Speech page, but the wizard
   itself should see the user through to a working install at least once.

### Phase C — Hands-free unification (G2)

If we adopt double-tap (recommended):

1. Implement a tap-window detector in `hotkey/manager.rs` that promotes a
   second press of the record hotkey within ~300ms into a hands-free toggle.
2. Remove `hotkey_hands_free` from defaults, schema seeds, registration code,
   and the Hotkeys UI.
3. Update the Hotkeys page copy to describe the double-tap behavior on the
   record entry.
4. Add a regression test in the hotkey state machine for the tap window.

If we keep the separate hotkey, instead update `docs/PRODUCT_AUDIT.md` and
the spec text to reflect that decision.

### Phase D — Dashboard polish (G4)

1. Change the four stat cards to render a true empty state row with the
   "what will appear here" copy from the spec.
2. Surface the active model name and size on `ModelStatusCard` or move that
   info into the existing card without reflowing the layout.
3. Add a small "All processing on this device" line near the top of the
   Dashboard (G7).

### Phase E — Tray pause reflection (G8)

1. Pass the paused state from the backend to the frontend (Tauri event or
   query).
2. Show a chip in the sidebar header / window title when paused.
3. Optional: flash the overlay briefly on a hotkey press while paused.

### Phase F — Hotkey reservation warnings (G5)

Lower priority. Can land in a later patch release. Add a curated reserved
list per platform and render a soft warning under the rebind input when the
captured combo matches.

---

## 5. Resolved product questions

| Question | Resolution |
|---|---|
| Hands-free model — double-tap or separate hotkey? | Double-tap of the record hotkey. The separate `Ctrl+F8` was removed and the legacy setting is migrated out. |
| Skip setup button? | Kept as an escape hatch. Users who skip see a "Speech files aren't installed" banner on the Dashboard with a one-click install CTA. |
| Long product spec? | Treated as a historical artifact. `README.md` + `docs/PRODUCT_AUDIT.md` are the source of truth. |

---

## 6. What this audit deliberately does **not** propose

- Re-adding the corrections engine, even a non-LLM pattern-based one.
- Re-adding the dictionary page, training paragraphs, or any learning
  surface.
- Re-adding Ollama, BYOK, or any external speech provider.
- Re-adding processing modes or app profile routing.
- Adding a local LLM for cleanup.

These items are listed here only so future readers know they were considered
and explicitly declined as part of the current product direction.
