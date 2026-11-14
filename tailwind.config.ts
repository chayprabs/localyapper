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
        accent: token("accent"),
        ok: token("ok"),
        warn: token("warn"),
        danger: token("danger"),
      },
      transitionTimingFunction: {
        out: "cubic-bezier(0.2, 0.8, 0.2, 1)",
        spring: "cubic-bezier(0.3, 1.4, 0.6, 1)",
      },
      animation: {
        "rise-in": "rise-in 420ms cubic-bezier(0.2, 0.8, 0.2, 1) both",
        "fade-in": "fade-in 240ms ease-out both",
        ring: "ring 3.2s cubic-bezier(0.2, 0.6, 0.3, 1) infinite",
      },
    },
  },
  plugins: [],
} satisfies Config;
