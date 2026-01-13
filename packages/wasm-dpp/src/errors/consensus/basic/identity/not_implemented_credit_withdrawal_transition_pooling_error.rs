use dpp::consensus::basic::identity::NotImplementedCreditWithdrawalTransitionPoolingError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=NotImplementedCreditWithdrawalTransitionPoolingError)]
pub struct NotImplementedCreditWithdrawalTransitionPoolingErrorWasm {
    inner: NotImplementedCreditWithdrawalTransitionPoolingError,
}

impl From<&NotImplementedCreditWithdrawalTransitionPoolingError>
    for NotImplementedCreditWithdrawalTransitionPoolingErrorWasm
{
    fn from(e: &NotImplementedCreditWithdrawalTransitionPoolingError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=NotImplementedCreditWithdrawalTransitionPoolingError)]
impl NotImplementedCreditWithdrawalTransitionPoolingErrorWasm {
    #[wasm_bindgen(js_name=getPooling)]
    pub fn pooling(&self) -> u8 {
        self.inner.pooling()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
