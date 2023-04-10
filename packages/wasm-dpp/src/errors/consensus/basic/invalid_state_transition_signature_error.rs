use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::InvalidStateTransitionSignatureError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidStateTransitionSignatureError)]
pub struct InvalidStateTransitionSignatureErrorWasm {
    inner: InvalidStateTransitionSignatureError,
}

impl From<&InvalidStateTransitionSignatureError> for InvalidStateTransitionSignatureErrorWasm {
    fn from(e: &InvalidStateTransitionSignatureError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidStateTransitionSignatureError)]
impl InvalidStateTransitionSignatureErrorWasm {
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
