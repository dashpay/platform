use dpp::consensus::basic::UnsupportedProtocolVersionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=UnsupportedProtocolVersionError)]
pub struct UnsupportedProtocolVersionErrorWasm {
    inner: UnsupportedProtocolVersionError,
}

impl From<&UnsupportedProtocolVersionError> for UnsupportedProtocolVersionErrorWasm {
    fn from(e: &UnsupportedProtocolVersionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=UnsupportedProtocolVersionError)]
impl UnsupportedProtocolVersionErrorWasm {
    #[wasm_bindgen(js_name=getParsedProtocolVersion)]
    pub fn parsed_protocol_version(&self) -> u32 {
        self.inner.parsed_protocol_version()
    }

    #[wasm_bindgen(js_name=getLatestVersion)]
    pub fn latest_version(&self) -> u32 {
        self.inner.latest_version()
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
