import type { Config } from "tailwindcss";

// Colours are RGB channels in CSS variables (see src/index.css) so opacity
// modifiers such as text-ink/50 work in both themes.
const token = (name: string) => `rgb(var(--${name}) / <alpha-value>)`;

export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Text",
          "Segoe UI Variable Text",
          "Segoe UI",
          "Inter",
          "Roboto",
          "Ubuntu",
          "sans-serif",
        ],
        mono: ["ui-monospace", "SF Mono", "Cascadia Mono", "Menlo", "Consolas", "monospace"],
      },
      colors: {
        // "ink" is the text colour: near-black in light, near-white in dark.
        ink: token("ink"),
        canvas: token("canvas"),
        // The only colour: recording, errors and destructive actions.
        signal: token("signal"),
      },
      animation: {
        "fade-in": "fade-in 140ms ease-out both",
      },
    },
  },
  plugins: [],
} satisfies Config;
