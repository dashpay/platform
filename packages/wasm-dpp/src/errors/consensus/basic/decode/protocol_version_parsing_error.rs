use crate::buffer::Buffer;

use dpp::consensus::basic::decode::ProtocolVersionParsingError;
use dpp::errors::consensus::codes::ErrorWithCode;
use dpp::errors::consensus::ConsensusError;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ProtocolVersionParsingError)]
pub struct ProtocolVersionParsingErrorWasm {
    inner: ProtocolVersionParsingError,
}

impl From<&ProtocolVersionParsingError> for ProtocolVersionParsingErrorWasm {
    fn from(e: &ProtocolVersionParsingError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ProtocolVersionParsingError)]
impl ProtocolVersionParsingErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(parsing_error: String) -> Self {
        Self {
            inner: ProtocolVersionParsingError::new(parsing_error),
        }
    }

    #[wasm_bindgen(js_name = getParsingError)]
    pub fn get_parsing_error(&self) -> String {
        self.inner.parsing_error().to_string()
    }

    #[wasm_bindgen(js_name = getCode)]
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
            .serialize_to_bytes_with_platform_version(PlatformVersion::first())
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
