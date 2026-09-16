use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ReferencedEntityNotFoundError)]
pub struct ReferencedEntityNotFoundErrorWasm {
    inner: ReferencedEntityNotFoundError,
}

impl From<&ReferencedEntityNotFoundError> for ReferencedEntityNotFoundErrorWasm {
    fn from(e: &ReferencedEntityNotFoundError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ReferencedEntityNotFoundError)]
impl ReferencedEntityNotFoundErrorWasm {
    #[wasm_bindgen(js_name=getEntityId)]
    pub fn entity_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.entity_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getEntityType)]
    pub fn entity_type(&self) -> String {
        self.inner.entity_type().to_string()
    }

    #[wasm_bindgen(js_name=getPath)]
    pub fn path(&self) -> String {
        self.inner.path().to_string()
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
