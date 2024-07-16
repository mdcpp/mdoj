/** @type {import('tailwindcss').Config} */
const colors = require('tailwindcss/colors')
module.exports = {
  content: {
    relative: true,
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    colors: {
      'text': '#e2e6ed',
      'primary': '#427fdc',
      'secondary': '#264e8a',
      'accent': '#9bb4d9',
      slate: colors.slate,
      red: colors.red,
      yellow: colors.yellow,
      green: colors.green,
    },
    container: {
      center: true,
      padding: {
        DEFAULT: '1rem',
        sm: '2rem',
        lg: '4rem',
        xl: '5rem',
      },
    },
    extend: {},
  },
  plugins: [],
}
