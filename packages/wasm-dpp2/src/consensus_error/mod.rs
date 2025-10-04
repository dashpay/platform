use crate::utils::WithJsError;
use dpp::consensus::ConsensusError;
use dpp::serialization::PlatformDeserializable;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "ConsensusError")]
pub struct ConsensusErrorWASM(ConsensusError);

#[wasm_bindgen(js_class = ConsensusError)]
impl ConsensusErrorWASM {
    #[wasm_bindgen(js_name = "deserialize")]
    pub fn deserialize(error: Vec<u8>) -> Result<Self, JsValue> {
        Ok(ConsensusErrorWASM(
            ConsensusError::deserialize_from_bytes(error.as_slice()).with_js_error()?,
        ))
    }

    #[wasm_bindgen(getter = "message")]
    pub fn message(&self) -> String {
        self.0.to_string()
    }
}
