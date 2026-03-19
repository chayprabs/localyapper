import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Inter",
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
      },
      colors: {
        "window-bg": "#f9f9f9",
        "sidebar-bg": "#eeeeee",
        "card-bg": "#FFFFFF",
        primary: "#0058bc",
        "primary-tint": "rgba(0,88,188,0.12)",
        success: "#006b19",
        destructive: "#ba1a1a",
        "card-border": "rgba(0,0,0,0.07)",
        "text-secondary": "rgba(0,0,0,0.50)",
        "text-tertiary": "rgba(0,0,0,0.26)",
        "text-label": "rgba(0,0,0,0.26)",
        separator: "rgba(0,0,0,0.08)",
      },
      keyframes: {
        "waveform-bar": {
          "0%, 100%": { transform: "scaleY(0.3)" },
          "50%": { transform: "scaleY(1)" },
        },
        "pulse-subtle": {
          "0%, 100%": { transform: "scale(1)" },
          "50%": { transform: "scale(1.15)" },
        },
        spin: {
          "0%": { transform: "rotate(0deg)" },
          "100%": { transform: "rotate(360deg)" },
        },
      },
      animation: {
        "waveform-1": "waveform-bar 1.2s ease-in-out infinite 0s",
        "waveform-2": "waveform-bar 1.2s ease-in-out infinite 0.1s",
        "waveform-3": "waveform-bar 1.2s ease-in-out infinite 0.2s",
        "waveform-4": "waveform-bar 1.2s ease-in-out infinite 0.3s",
        "waveform-5": "waveform-bar 1.2s ease-in-out infinite 0.4s",
        "pulse-subtle": "pulse-subtle 2s ease-in-out infinite",
        "spin-slow": "spin 1.5s linear infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
