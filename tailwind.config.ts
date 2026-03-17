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
        "window-bg": "#EDEDED",
        "sidebar-bg": "#EBEBEB",
        "card-bg": "#FFFFFF",
        primary: "#007AFF",
        success: "#28CD41",
        destructive: "#FF3B30",
        "text-primary": "#1C1C1E",
      },
    },
  },
  plugins: [],
} satisfies Config;
