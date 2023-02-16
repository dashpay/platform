use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionPoolingError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityCreditWithdrawalTransitionPoolingError)]
pub struct InvalidIdentityCreditWithdrawalTransitionPoolingErrorWasm {
    inner: InvalidIdentityCreditWithdrawalTransitionPoolingError,
}

impl From<&InvalidIdentityCreditWithdrawalTransitionPoolingError>
    for InvalidIdentityCreditWithdrawalTransitionPoolingErrorWasm
{
    fn from(e: &InvalidIdentityCreditWithdrawalTransitionPoolingError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityCreditWithdrawalTransitionPoolingError)]
impl InvalidIdentityCreditWithdrawalTransitionPoolingErrorWasm {
    #[wasm_bindgen(js_name=getPooling)]
    pub fn pooling(&self) -> u8 {
        self.inner.pooling()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}
