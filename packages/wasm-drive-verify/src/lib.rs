use wasm_bindgen::prelude::*;

pub mod contract;
pub mod document;
pub mod identity;
pub mod single_document;
pub mod system;
pub mod group;
pub mod state_transition;
pub mod tokens;
pub mod voting;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}