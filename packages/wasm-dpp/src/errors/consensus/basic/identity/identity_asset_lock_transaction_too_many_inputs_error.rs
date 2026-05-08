use dpp::consensus::basic::identity::IdentityAssetLockTransactionTooManyInputsError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityAssetLockTransactionTooManyInputsError)]
pub struct IdentityAssetLockTransactionTooManyInputsErrorWasm {
    inner: IdentityAssetLockTransactionTooManyInputsError,
}

impl From<&IdentityAssetLockTransactionTooManyInputsError>
    for IdentityAssetLockTransactionTooManyInputsErrorWasm
{
    fn from(e: &IdentityAssetLockTransactionTooManyInputsError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockTransactionTooManyInputsError)]
impl IdentityAssetLockTransactionTooManyInputsErrorWasm {
    #[wasm_bindgen(js_name=getMaxInputs)]
    pub fn get_max_inputs(&self) -> u16 {
        self.inner.max_inputs()
    }

    #[wasm_bindgen(js_name=getActualInputs)]
    pub fn get_actual_inputs(&self) -> u16 {
        self.inner.actual_inputs()
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
