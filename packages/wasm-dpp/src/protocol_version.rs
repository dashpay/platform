use wasm_bindgen::prelude::wasm_bindgen;
use dpp::version::LATEST_VERSION;

#[wasm_bindgen(js_name = getLatestProtocolVersion)]
pub fn latest_protocol_version() -> u32 {
    LATEST_VERSION
}