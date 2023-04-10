use crate::buffer::Buffer;
use dpp::consensus::basic::data_contract::InvalidJsonSchemaRefError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidJsonSchemaRefError)]
pub struct InvalidJsonSchemaRefErrorWasm {
    inner: InvalidJsonSchemaRefError,
}

impl From<&InvalidJsonSchemaRefError> for InvalidJsonSchemaRefErrorWasm {
    fn from(e: &InvalidJsonSchemaRefError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidJsonSchemaRefError)]
impl InvalidJsonSchemaRefErrorWasm {
    #[wasm_bindgen(js_name=getRefError)]
    pub fn get_ref_error(&self) -> String {
        self.inner.message()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name=serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize()
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
