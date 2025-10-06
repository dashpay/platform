use crate::error::WasmDppResult;
use dpp::consensus::ConsensusError;
use dpp::serialization::PlatformDeserializable;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "ConsensusError")]
pub struct ConsensusErrorWasm(ConsensusError);

#[wasm_bindgen(js_class = ConsensusError)]
impl ConsensusErrorWasm {
    #[wasm_bindgen(js_name = "deserialize")]
    pub fn deserialize(error: Vec<u8>) -> WasmDppResult<Self> {
        Ok(ConsensusErrorWasm(ConsensusError::deserialize_from_bytes(
            error.as_slice(),
        )?))
    }

    #[wasm_bindgen(getter = "message")]
    pub fn message(&self) -> String {
        self.0.to_string()
    }
}
