pub mod app;
pub mod components;
pub mod pages;
pub mod utils;

use cfg_if::cfg_if;
#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
/// SAFETY: leptos use single threaded
/// Change to lock allocator when we have multithread in web
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

cfg_if! {
if #[cfg(feature = "hydrate")] {

  use wasm_bindgen::prelude::wasm_bindgen;

  #[wasm_bindgen]
  pub fn hydrate() {
    use app::*;

    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
  }
}
}
