use crate::buffer::Buffer;
use dpp::consensus::basic::UnsupportedVersionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = UnsupportedVersionError)]
pub struct UnsupportedVersionErrorWasm {
    inner: UnsupportedVersionError,
}

impl From<&UnsupportedVersionError> for UnsupportedVersionErrorWasm {
    fn from(e: &UnsupportedVersionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class = UnsupportedVersionError)]
impl UnsupportedVersionErrorWasm {
    #[wasm_bindgen(js_name = getReceivedVersion)]
    pub fn received_version(&self) -> u16 {
        self.inner.received_version()
    }

    #[wasm_bindgen(js_name = getMinVersion)]
    pub fn min_version(&self) -> u16 {
        self.inner.min_version()
    }

    #[wasm_bindgen(js_name = getMaxVersion)]
    pub fn max_version(&self) -> u16 {
        self.inner.max_version()
    }

    #[wasm_bindgen(js_name = getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize()
            .map_err(|e| JsError::from(e))?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
