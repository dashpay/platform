use dash_sdk::Error;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(thiserror::Error, Debug)]
#[error("Dash SDK error: {0:?}")]
pub struct WasmError(#[from] Error);
