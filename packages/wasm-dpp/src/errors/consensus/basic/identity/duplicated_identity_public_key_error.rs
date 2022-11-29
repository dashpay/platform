use dpp::consensus::basic::identity::{DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError};
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyError)]
pub struct DuplicatedIdentityPublicKeyErrorWasm {
    inner: DuplicatedIdentityPublicKeyError
}

impl From<&DuplicatedIdentityPublicKeyError> for DuplicatedIdentityPublicKeyErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyError) -> Self {
        Self {
            inner: e.clone()
        }
    }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyError)]
impl DuplicatedIdentityPublicKeyErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedPublicKeysIds)]
    pub fn duplicated_public_keys_ids(&self) -> Vec<u32> {
        // TODO: key ids probably should be u32
        self.inner.duplicated_ids().iter().map(|id| *id as u32).collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}