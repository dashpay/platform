use dpp::consensus::basic::identity::InvalidIdentityCreditTransferAmountError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityCreditTransferAmountError)]
pub struct InvalidIdentityCreditTransferAmountErrorWasm {
    inner: InvalidIdentityCreditTransferAmountError,
}

impl From<&InvalidIdentityCreditTransferAmountError>
    for InvalidIdentityCreditTransferAmountErrorWasm
{
    fn from(e: &InvalidIdentityCreditTransferAmountError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityCreditTransferAmountError)]
impl InvalidIdentityCreditTransferAmountErrorWasm {
    #[wasm_bindgen(js_name=getAmount)]
    pub fn amount(&self) -> u64 {
        self.inner.amount()
    }

    #[wasm_bindgen(js_name=getMinAmount)]
    pub fn min_amount(&self) -> u64 {
        self.inner.min_amount()
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
