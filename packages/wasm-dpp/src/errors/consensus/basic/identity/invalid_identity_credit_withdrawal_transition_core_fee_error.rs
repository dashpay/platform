use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionCoreFeeError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityCreditWithdrawalTransitionCoreFeeError)]
pub struct InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm {
    inner: InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
}

impl From<&InvalidIdentityCreditWithdrawalTransitionCoreFeeError>
    for InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm
{
    fn from(e: &InvalidIdentityCreditWithdrawalTransitionCoreFeeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityCreditWithdrawalTransitionCoreFeeError)]
impl InvalidIdentityCreditWithdrawalTransitionCoreFeeErrorWasm {
    #[wasm_bindgen(js_name=getCoreFee)]
    pub fn core_fee_per_byte(&self) -> u32 {
        self.inner.core_fee_per_byte()
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
