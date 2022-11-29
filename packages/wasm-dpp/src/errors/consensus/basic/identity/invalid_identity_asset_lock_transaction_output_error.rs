use dpp::consensus::basic::identity::{DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError, InvalidIdentityAssetLockTransactionOutputError};
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityAssetLockTransactionOutputError)]
pub struct InvalidIdentityAssetLockTransactionOutputErrorWasm {
    inner: InvalidIdentityAssetLockTransactionOutputError
}

impl From<&InvalidIdentityAssetLockTransactionOutputError> for InvalidIdentityAssetLockTransactionOutputErrorWasm {
    fn from(e: &InvalidIdentityAssetLockTransactionOutputError) -> Self {
        Self {
            inner: e.clone()
        }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityAssetLockTransactionOutputError)]
impl InvalidIdentityAssetLockTransactionOutputErrorWasm {
    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn output_index(&self) -> usize {
        self.inner.output_index()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}