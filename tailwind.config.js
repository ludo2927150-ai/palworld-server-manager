/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: { extend: { colors: { slate: { 50: "rgb(var(--s-50) / <alpha-value>)",100: "rgb(var(--s-100) / <alpha-value>)",200: "rgb(var(--s-200) / <alpha-value>)",300: "rgb(var(--s-300) / <alpha-value>)",400: "rgb(var(--s-400) / <alpha-value>)",500: "rgb(var(--s-500) / <alpha-value>)",600: "rgb(var(--s-600) / <alpha-value>)",700: "rgb(var(--s-700) / <alpha-value>)",800: "rgb(var(--s-800) / <alpha-value>)",900: "rgb(var(--s-900) / <alpha-value>)",950: "rgb(var(--s-950) / <alpha-value>)" }, pal: { 500: "#2dd4bf", 600: "#14b8a6" } } } },
  plugins: [],
};
