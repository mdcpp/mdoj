pub mod app;
pub mod components;
pub mod config;
pub mod errors;
pub mod grpc;
pub mod pages;
pub mod session;
pub mod utils;

use cfg_if::cfg_if;

cfg_if! {
if #[cfg(feature = "hydrate")] {

  use wasm_bindgen::prelude::wasm_bindgen;

  #[wasm_bindgen]
  pub fn hydrate() {
    use app::*;
    use leptos::*;

    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
  }
}
}
