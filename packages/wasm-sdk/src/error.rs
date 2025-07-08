use dash_sdk::Error;
use std::fmt::Display;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

#[wasm_bindgen]
#[derive(thiserror::Error, Debug)]
#[error("Dash SDK error: {0:?}")]
pub struct WasmError(#[from] Error);

pub(crate) fn to_js_error(e: impl Display) -> JsError {
    JsError::new(&format!("{}", e))
}
