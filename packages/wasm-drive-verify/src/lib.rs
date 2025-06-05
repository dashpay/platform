use wasm_bindgen::prelude::*;

pub mod contract;
pub mod document;
pub mod group;
pub mod identity;
pub mod single_document;
pub mod state_transition;
pub mod system;
pub mod tokens;
pub mod voting;

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
