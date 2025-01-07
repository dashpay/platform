use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

pub mod context_provider;
pub mod dpp;
pub mod error;
pub mod sdk;
pub mod state_transitions;
pub mod verify;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// TODO: Remove tracing or use
//https://github.com/old-storyai/tracing-wasm
//https://github.com/jquesada2016/tracing_subscriber_wasm
//https://crates.io/crates/tracing-web

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    Ok(())
}
