/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: { extend: { colors: { pal: { 500: "#2dd4bf", 600: "#14b8a6" } } } },
  plugins: [],
};
