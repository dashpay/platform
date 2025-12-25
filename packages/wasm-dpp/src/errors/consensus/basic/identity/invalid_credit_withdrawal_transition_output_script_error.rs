use dpp::consensus::basic::identity::InvalidCreditWithdrawalTransitionOutputScriptError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidCreditWithdrawalTransitionOutputScriptError)]
pub struct InvalidCreditWithdrawalTransitionOutputScriptErrorWasm {
    inner: InvalidCreditWithdrawalTransitionOutputScriptError,
}

impl From<&InvalidCreditWithdrawalTransitionOutputScriptError>
    for InvalidCreditWithdrawalTransitionOutputScriptErrorWasm
{
    fn from(e: &InvalidCreditWithdrawalTransitionOutputScriptError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidCreditWithdrawalTransitionOutputScriptError)]
impl InvalidCreditWithdrawalTransitionOutputScriptErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
