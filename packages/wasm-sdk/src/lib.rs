use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

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
    let dash_sdk = sdk::WasmSdkBuilder::new_mainnet();
    let sdk = dash_sdk
        .with_context_provider(verify::WasmContext {})
        .build()
        .expect("build sdk");

    sdk::identity_fetch(&sdk).await;
    Ok(())
}
