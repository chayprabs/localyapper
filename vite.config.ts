import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

const host = process.env.TAURI_DEV_HOST;

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
  envPrefix: ["VITE_", "TAURI_"],
  esbuild: {
    // Strip dev-only diagnostics from production bundles. Keep them in
    // TAURI_DEBUG builds so console.error/warn remain visible.
    drop: process.env.TAURI_DEBUG ? [] : ["console", "debugger"],
  },
  build: {
    target:
      process.env.TAURI_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    cssCodeSplit: true,
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
