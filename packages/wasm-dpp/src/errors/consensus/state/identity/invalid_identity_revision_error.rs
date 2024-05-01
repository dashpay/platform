use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::ConsensusError;

use js_sys::Number;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityRevisionError)]
pub struct InvalidIdentityRevisionErrorWasm {
    inner: InvalidIdentityRevisionError,
}

impl From<&InvalidIdentityRevisionError> for InvalidIdentityRevisionErrorWasm {
    fn from(e: &InvalidIdentityRevisionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityRevisionError)]
impl InvalidIdentityRevisionErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn identity_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.identity_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getCurrentRevision)]
    pub fn current_revision(&self) -> Number {
        // It might be overflow
        Number::from(*self.inner.current_revision() as f64)
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
