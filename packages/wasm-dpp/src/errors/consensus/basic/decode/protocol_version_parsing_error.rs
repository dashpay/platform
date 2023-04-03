use crate::buffer::Buffer;
use dpp::consensus::basic::decode::ProtocolVersionParsingError;
use dpp::errors::consensus::codes::ErrorWithCode;
use dpp::errors::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ProtocolVersionParsingError)]
pub struct ProtocolVersionParsingErrorWasm {
    inner: ProtocolVersionParsingError,
}

impl ProtocolVersionParsingErrorWasm {
    pub fn new(error: &ProtocolVersionParsingError) -> Self {
        Self {
            inner: error.clone(),
        }
    }
}

#[wasm_bindgen(js_class=ProtocolVersionParsingError)]
impl ProtocolVersionParsingErrorWasm {
    #[wasm_bindgen(js_name = getParsingError)]
    pub fn get_parsing_error(&self) -> String {
        self.inner.parsing_error().to_string()
    }

    #[wasm_bindgen(js_name = getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(js_name = serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize()
            .map_err(|e| JsError::from(e))?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
