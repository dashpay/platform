use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;
use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityAlreadyExistsError)]
pub struct IdentityAlreadyExistsErrorWasm {
    inner: IdentityAlreadyExistsError,
}

impl From<&IdentityAlreadyExistsError>
for IdentityAlreadyExistsErrorWasm
{
    fn from(e: &IdentityAlreadyExistsError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAlreadyExistsError)]
impl IdentityAlreadyExistsErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn identity_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.identity_id())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}
