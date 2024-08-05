pub mod app;
pub mod components;
pub mod config;
pub mod errors;
pub mod grpc;
pub mod pages;
pub mod session;

use cfg_if::cfg_if;
#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

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
