use dpp::version::{PlatformVersion, LATEST_VERSION};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = getLatestProtocolVersion)]
pub fn latest_protocol_version() -> u32 {
    PlatformVersion::latest().protocol_version
}
