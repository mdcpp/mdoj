# MDOJ Frontend

This project use bleeding-edge framework call Leptos and tailwind/sass for styling.

- [Color Palette](https://www.realtimecolors.com/?colors=e2e6ed-0d121a-9bb4d9-264e8a-427fdc)
- [Leptos](https://leptos.dev/)

## Pre-requirement

This project use nightly Rust and sass, so you need install following dependency

- `rustup toolchain install nightly --allow-downgrade` - make sure you have Rust nightly
- `rustup target add wasm32-unknown-unknown` - add the ability to compile Rust to WebAssembly
- `npm install -g sass` - install `dart-sass`

## Development

Run `just dev` to start frontend
By default, it will running `http://localhost:3000`

Run `just setup-backend` to run backend for development
