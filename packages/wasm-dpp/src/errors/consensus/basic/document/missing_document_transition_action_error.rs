use crate::buffer::Buffer;
use dpp::consensus::basic::document::MissingDocumentTransitionActionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::serialization::serialization_traits::PlatformSerializable;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDocumentTransitionActionError)]
pub struct MissingDocumentTransitionActionErrorWasm {
    inner: MissingDocumentTransitionActionError,
}

impl From<&MissingDocumentTransitionActionError> for MissingDocumentTransitionActionErrorWasm {
    fn from(e: &MissingDocumentTransitionActionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=MissingDocumentTransitionActionError)]
impl MissingDocumentTransitionActionErrorWasm {
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
