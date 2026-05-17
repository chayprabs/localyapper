// Overlay window entry point. Stays separate from main.tsx so the overlay
// WebView does not download or parse the settings/wizard module graph.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Overlay } from "@/components/overlay/Overlay";
import "./index.css";

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("Root element not found");

createRoot(rootEl).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
);
