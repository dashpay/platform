use dpp::consensus::basic::identity::InvalidIdentityAssetLockTransactionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityAssetLockTransactionError)]
pub struct InvalidIdentityAssetLockTransactionErrorWasm {
    inner: InvalidIdentityAssetLockTransactionError,
}

impl From<&InvalidIdentityAssetLockTransactionError>
    for InvalidIdentityAssetLockTransactionErrorWasm
{
    fn from(e: &InvalidIdentityAssetLockTransactionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityAssetLockTransactionError)]
impl InvalidIdentityAssetLockTransactionErrorWasm {
    #[wasm_bindgen(js_name=getErrorMessage)]
    pub fn get_error_message(&self) -> String {
        self.inner.message().to_string()
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
