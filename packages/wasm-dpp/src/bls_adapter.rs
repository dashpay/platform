// extern "C" {
//     #[wasm_bindgen(js_name = "default")]
//     type Web3;
//
//     #[wasm_bindgen(constructor, js_class = "default")]
//     fn new(_: &Provider) -> Web3;
//
//     #[wasm_bindgen(static_method_of = Web3, getter, js_class = "default")]
//     fn givenProvider() -> Provider;
//
//     type Provider;
// }

// use wasm_bindgen::prelude::wasm_bindgen;

use wasm_bindgen::prelude::*;
use dpp::{BlsValidator, PublicKeyValidationError};

#[wasm_bindgen]
extern "C" {
    pub type BlsAdapter;

    #[wasm_bindgen(method)]
    pub fn validate_public_key(this: &BlsAdapter, pk: &[u8]) -> bool;

    // #[wasm_bindgen(constructor)]
    // pub fn new() -> Buffer;

    // #[wasm_bindgen(constructor, js_name = "from")]
    // pub fn from_bytes(js_sys: &[u8]) -> Buffer;
    //
    // #[wasm_bindgen(constructor, js_name = "from")]
    // pub fn from_string(js_sys: String) -> Buffer;
}

pub struct BlsAdapterRust(pub BlsAdapter);

impl BlsValidator for BlsAdapterRust {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError> {
        let is_valid = self.0.validate_public_key(pk);

        if !is_valid {
            return Err(PublicKeyValidationError::new("Failed"))
        }

        Ok(())
    }
}