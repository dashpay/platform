use dpp::version::LATEST_VERSION;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = getLatestProtocolVersion)]
pub fn latest_protocol_version() -> u32 {
    LATEST_VERSION
}
