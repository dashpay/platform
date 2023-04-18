use crate::buffer::Buffer;
use dpp::consensus::basic::data_contract::DuplicateIndexNameError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicateIndexNameError)]
pub struct DuplicateIndexNameErrorWasm {
    inner: DuplicateIndexNameError,
}

impl From<&DuplicateIndexNameError> for DuplicateIndexNameErrorWasm {
    fn from(e: &DuplicateIndexNameError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicateIndexNameError)]
impl DuplicateIndexNameErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type().to_string()
    }

    #[wasm_bindgen(js_name=getDuplicateIndexName)]
    pub fn get_duplicate_index_name(&self) -> String {
        self.inner.duplicate_index_name().to_string()
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
