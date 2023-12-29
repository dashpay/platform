use dpp::consensus::basic::IncompatibleProtocolVersionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IncompatibleProtocolVersionError)]
pub struct IncompatibleProtocolVersionErrorWasm {
    inner: IncompatibleProtocolVersionError,
}

impl From<&IncompatibleProtocolVersionError> for IncompatibleProtocolVersionErrorWasm {
    fn from(e: &IncompatibleProtocolVersionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IncompatibleProtocolVersionError)]
impl IncompatibleProtocolVersionErrorWasm {
    #[wasm_bindgen(js_name=getParsedProtocolVersion)]
    pub fn parsed_protocol_version(&self) -> u32 {
        self.inner.parsed_protocol_version()
    }

    #[wasm_bindgen(js_name=getMinimalProtocolVersion)]
    pub fn minimal_protocol_version(&self) -> u32 {
        self.inner.minimal_protocol_version()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
