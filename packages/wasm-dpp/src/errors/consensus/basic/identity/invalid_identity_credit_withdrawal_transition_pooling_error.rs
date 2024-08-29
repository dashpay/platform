use dpp::consensus::basic::identity::NotImplementedIdentityCreditWithdrawalTransitionPoolingError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=NotImplementedIdentityCreditWithdrawalTransitionPoolingError)]
pub struct NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm {
    inner: NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
}

impl From<&NotImplementedIdentityCreditWithdrawalTransitionPoolingError>
    for NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm
{
    fn from(e: &NotImplementedIdentityCreditWithdrawalTransitionPoolingError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=NotImplementedIdentityCreditWithdrawalTransitionPoolingError)]
impl NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm {
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
