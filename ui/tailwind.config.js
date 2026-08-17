/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        proponent: {
          light: '#60a5fa',
          DEFAULT: '#2563eb',
          dark: '#1e40af',
        },
        opponent: {
          light: '#fb923c',
          DEFAULT: '#ea580c',
          dark: '#9a3412',
        },
        arena: {
          bg: '#0f172a',
          card: '#1e293b',
          border: '#334155',
          text: '#f8fafc',
          muted: '#94a3b8',
        }
      }
    },
  },
  plugins: [],
}
