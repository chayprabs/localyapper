import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

const host = process.env.TAURI_DEV_HOST;
// Set by the Tauri CLI for `tauri dev` and `tauri build --debug`.
const debug = process.env.TAURI_ENV_DEBUG === "true";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri 2 names its variables TAURI_ENV_*. Nothing broader is exposed,
  // so a TAURI_SIGNING_* secret in CI can never reach the bundle.
  envPrefix: ["VITE_", "TAURI_ENV_"],
  esbuild: {
    // Strip dev-only diagnostics from release bundles. Keep them in
    // debug builds (`tauri dev`, `tauri build --debug`) so console
    // output remains visible.
    drop: debug ? [] : ["console", "debugger"],
  },
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: debug ? false : "esbuild",
    sourcemap: debug,
    cssCodeSplit: true,
    // Assets stay files: the CSP allows images from 'self' only, so a
    // small image inlined as a data: URI (the title bar logo) would be
    // blocked.
    assetsInlineLimit: 0,
    rollupOptions: {
      input: {
        // Settings + wizard live behind index.html
        main: path.resolve(__dirname, "index.html"),
        // Overlay pill loads a tiny separate bundle so the floating
        // window does not parse the settings module graph.
        overlay: path.resolve(__dirname, "overlay.html"),
      },
      output: {
        // Pull React + ReactDOM into their own vendor chunk so the
        // bundle hash stays stable across app changes (better cache
        // hit rate across releases) and the parse cost is paid once
        // even if both WebViews boot.
        manualChunks(id) {
          if (id.includes("node_modules/react/")) return "react-vendor";
          if (id.includes("node_modules/react-dom/")) return "react-vendor";
          if (id.includes("node_modules/scheduler/")) return "react-vendor";
          return undefined;
        },
      },
    },
  },
});
