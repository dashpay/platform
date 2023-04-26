use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::BasicECDSAError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=BasicECDSAError)]
pub struct BasicECDSAErrorWasm {
    inner: BasicECDSAError,
}

impl From<&BasicECDSAError> for BasicECDSAErrorWasm {
    fn from(e: &BasicECDSAError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=BasicECDSAError)]
impl BasicECDSAErrorWasm {
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
            .map_err(|e| JsError::from(e))?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
