// Overlay page. Rust owns the window (position, show, hide); this page only
// renders the view it is sent. It listens for `overlay-view` and
// `overlay-level`, and sends back only the copy and dismiss clicks.
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
  if (window.__LY_TEST__) void invoke("test_overlay_seen", { kind: view.kind, level: null });
}

function setLevel(level: number): void {
  const v = Math.max(0, Math.min(1, level));
  bars.forEach((bar, i) => {
    bar.style.transform = `scaleY(${(0.15 + 0.85 * v * (weights[i] ?? 1)).toFixed(3)})`;
  });
}

void listen<View>("overlay-view", (e) => render(e.payload));
void listen<number>("overlay-level", (e) => {
  setLevel(e.payload);
  if (window.__LY_TEST__) void invoke("test_overlay_seen", { kind: null, level: e.payload });
});

document.getElementById("copy")?.addEventListener("click", () => {
  void invoke("overlay_copy");
});
document.getElementById("dismiss")?.addEventListener("click", () => {
  void invoke("overlay_dismiss");
});
