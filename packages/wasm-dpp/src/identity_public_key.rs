pub use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use dpp::identity::IdentityPublicKey;

#[wasm_bindgen(js_name=IdentityPublicKey)]
pub struct IdentityPublicKeyWasm(IdentityPublicKey);

// TODO

#[wasm_bindgen(js_class = IdentityPublicKey)]
impl IdentityPublicKeyWasm {}

impl std::convert::From<IdentityPublicKey> for IdentityPublicKeyWasm {
    fn from(v: IdentityPublicKey) -> Self {
        IdentityPublicKeyWasm(v)
    }
}
