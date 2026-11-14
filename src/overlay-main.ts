// Overlay page. Rust owns the window (position, show, hide); this page only
// renders the view it is sent. It listens for `overlay-view` and
// `overlay-level`, and sends back the copy and dismiss clicks and the
// drag of the pill (Rust moves the window and remembers the new place).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./overlay.css";

type View =
  | { kind: "hidden" }
  | { kind: "listening" }
  | { kind: "hands_free" }
  | { kind: "stopping_soon"; remaining_ms: number }
  | { kind: "processing" }
  | { kind: "done"; text: string }
  | { kind: "no_speech" }
  | { kind: "error"; message: string; text: string | null };

declare global {
  interface Window {
    __LY_TEST__?: boolean;
  }
}

const pill = document.getElementById("pill") as HTMLDivElement;
const label = document.getElementById("label") as HTMLSpanElement;
const text = document.getElementById("text") as HTMLSpanElement;
const bars = Array.from(pill.querySelectorAll<HTMLSpanElement>(".bars span"));
// Outer bars move less than the middle one.
const weights = [0.55, 0.8, 1, 0.8, 0.55];

function labelFor(view: View): string {
  switch (view.kind) {
    case "listening":
      return "Listening";
    case "hands_free":
      return "Hands-free";
    case "stopping_soon":
      return `Stopping in ${Math.ceil(view.remaining_ms / 1000)} s`;
    case "processing":
      return "Transcribing";
    case "done":
      return "";
    case "no_speech":
      return "No speech heard";
    case "error":
      return view.message;
    case "hidden":
      return "";
  }
}

function render(view: View): void {
  pill.dataset.kind = view.kind;
  label.textContent = labelFor(view);
  const shown =
    view.kind === "done" ? view.text : view.kind === "error" ? (view.text ?? "") : "";
  text.textContent = shown;
  pill.dataset.hasText = shown.length > 0 ? "true" : "false";
  if (view.kind !== "listening" && view.kind !== "hands_free" && view.kind !== "stopping_soon") {
    setLevel(0);
  }
  // A hidden window sends no pointer-up; Rust ends the drag as well.
  if (view.kind === "hidden") endDrag();
  if (window.__LY_TEST__) void invoke("test_overlay_seen", { kind: view.kind, level: null });
}

function setLevel(level: number): void {
  const v = Math.max(0, Math.min(1, level));
  bars.forEach((bar, i) => {
    bar.style.transform = `scaleY(${(0.15 + 0.85 * v * (weights[i] ?? 1)).toFixed(3)})`;
  });
}

let gotEvent = false;
void listen<View>("overlay-view", (e) => {
  gotEvent = true;
  render(e.payload);
}).then(() =>
  // A view sent before this listener existed would be lost: ask for it.
  invoke<View>("overlay_current_view").then((v) => {
    if (!gotEvent) render(v);
  }),
);
void listen<number>("overlay-level", (e) => {
  setLevel(e.payload);
  if (window.__LY_TEST__) void invoke("test_overlay_seen", { kind: null, level: e.payload });
});

// The pill follows the app's theme setting, like the settings window.
function applyTheme(theme: string): void {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.dataset.theme = theme;
  } else {
    delete root.dataset.theme;
  }
}
void invoke<string>("get_setting", { key: "theme" })
  .then(applyTheme)
  .catch(() => undefined);
void listen<string>("theme-changed", (e) => applyTheme(e.payload));

document.getElementById("copy")?.addEventListener("click", () => {
  void invoke("overlay_copy");
});
document.getElementById("dismiss")?.addEventListener("click", () => {
  void invoke("overlay_dismiss");
});

// Dragging the pill. The page only reports press, movement and release;
// Rust reads the real cursor position, so the window follows the cursor
// exactly on every monitor and scale factor. One `move` is in flight at
// a time, and the newest movement is never dropped.
let dragging = false;
let moveInFlight = false;
let movePending = false;

function sendMove(): void {
  if (!dragging) return;
  if (moveInFlight) {
    movePending = true;
    return;
  }
  moveInFlight = true;
  void invoke("overlay_drag", { phase: "move" })
    .catch(() => undefined)
    .finally(() => {
      moveInFlight = false;
      if (movePending) {
        movePending = false;
        sendMove();
      }
    });
}

function endDrag(): void {
  if (!dragging) return;
  dragging = false;
  movePending = false;
  pill.classList.remove("dragging");
  void invoke("overlay_drag", { phase: "end" });
}

pill.addEventListener("pointerdown", (e) => {
  if (e.button !== 0 || dragging) return;
  if (e.target instanceof Element && e.target.closest("button")) return;
  dragging = true;
  pill.classList.add("dragging");
  // Keeps the events coming when the cursor outruns the window.
  try {
    pill.setPointerCapture(e.pointerId);
  } catch {
    // Without capture the drag still works while the cursor is over the pill.
  }
  void invoke("overlay_drag", { phase: "start" });
  e.preventDefault();
});
pill.addEventListener("pointermove", sendMove);
pill.addEventListener("pointerup", endDrag);
pill.addEventListener("pointercancel", endDrag);
pill.addEventListener("lostpointercapture", endDrag);
