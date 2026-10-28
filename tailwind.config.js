/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Google Sans",
          "Google Sans Text",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
        mono: [
          "Google Sans Mono",
          "SF Mono",
          "Consolas",
          "monospace",
        ],
      },
      colors: {
        gemini: {
          bg: "#131314",
          surface: "#1e1f20",
          hover: "#282a2c",
          active: "#37393b",
          border: "rgba(255, 255, 255, 0.08)",
          text: "#e3e3e3",
          secondary: "#c4c7c5",
          muted: "#8e918f",
          blue: "#a8c7fa",
          blueHover: "#c2e7ff",
          blueDark: "#062e6f",
          blueContainer: "rgba(168, 199, 250, 0.12)",
          green: "#6dd58c",
          greenContainer: "rgba(109, 213, 140, 0.12)",
          amber: "#ffb74d",
          amberContainer: "rgba(255, 183, 77, 0.12)",
          red: "#f28b82",
          redContainer: "rgba(242, 139, 130, 0.12)",
        },
      },
      borderRadius: {
        "3xl": "24px",
        "2xl": "16px",
        "xl": "12px",
        pill: "9999px",
      },
      transitionTimingFunction: {
        material: "cubic-bezier(0.2, 0, 0, 1)",
      },
    },
  },
  plugins: [],
}
