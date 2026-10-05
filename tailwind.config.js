/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        ugent: {
          blue: "#1e64c8",
          darkblue: "#1e3a8a",
          navy: "#0f172a",
          card: "#182234",
          cardHover: "#1e2c44",
          accent: "#2563eb",
          yellow: "#ffd100",
        },
      },
    },
  },
  plugins: [],
}
