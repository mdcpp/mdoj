/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    relative: true,
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    colors: {
      'text': '#e8f7f0',
      'background': '#040a07',
      'primary': '#9addbc',
      'secondary': '#2b5280',
      'accent': '#606fca',
    },
    extend: {},
  },
  plugins: [],
}
