use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::document_timestamps_are_equal_error::DocumentTimestampsAreEqualError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentTimestampsAreEqualError)]
pub struct DocumentTimestampsAreEqualErrorWasm {
    inner: DocumentTimestampsAreEqualError,
}

impl From<&DocumentTimestampsAreEqualError> for DocumentTimestampsAreEqualErrorWasm {
    fn from(e: &DocumentTimestampsAreEqualError) -> Self {
        Self { inner: e.clone() }
    }
}
#[wasm_bindgen(js_class=DocumentTimestampsAreEqualError)]
impl DocumentTimestampsAreEqualErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
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
            .map_err(|e| JsError::from(e))?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
