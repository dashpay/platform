use dpp::errors::consensus::basic::JsonSchemaError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=JsonSchemaError)]
pub struct JsonSchemaErrorWasm {
    _inner: JsonSchemaError,
}

impl From<JsonSchemaError> for JsonSchemaErrorWasm {
    fn from(e: JsonSchemaError) -> Self {
        Self { _inner: e }
    }
}

// #[wasm_bindgen(js_class=JsonSchemaError)]
// impl ConsensusError for JsonSchemaErrorWasm {
//     fn get_code(&self) -> u32 {
//         DPPConsensusError::from(self.inner.clone()).get_code()
//     }
// }
