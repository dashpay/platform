extern crate web_sys;

pub use dash_platform_protocol::*;
pub use data_contract::*;
pub use document::*;
pub use identity::*;
pub use identity::*;
pub use identity_facade::*;
pub use identity_public_key::*;
pub use metadata::*;
pub use tx::*;

mod dash_platform_protocol;
mod data_contract;
mod document;
pub mod errors;
mod identifier;
mod identity;
mod identity_facade;
mod identity_public_key;
mod metadata;
pub mod mocks;

pub(crate) mod js_buffer;
mod tx;
mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    type Buffer;

    #[wasm_bindgen(constructor)]
    fn new() -> Buffer;

    #[wasm_bindgen(constructor, js_name = "from")]
    fn from_bytes(js_sys: &[u8]) -> Buffer;

    #[wasm_bindgen(constructor, js_name = "from")]
    fn from_string(js_sys: String) -> Buffer;
}
