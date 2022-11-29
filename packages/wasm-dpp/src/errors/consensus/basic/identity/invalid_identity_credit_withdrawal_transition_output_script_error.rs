use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionOutputScriptError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=InvalidIdentityCreditWithdrawalTransitionOutputScriptError)]
pub struct InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm {
    inner: InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
}

impl From<&InvalidIdentityCreditWithdrawalTransitionOutputScriptError>
for InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm
{
    fn from(e: &InvalidIdentityCreditWithdrawalTransitionOutputScriptError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityCreditWithdrawalTransitionOutputScriptError)]
impl InvalidIdentityCreditWithdrawalTransitionOutputScriptErrorWasm {
    #[wasm_bindgen(js_name=getOutputScript)]
    pub fn output_script(&self) -> Buffer {
        let script = self.inner.output_script().clone();
        Buffer::from_bytes(script.to_bytes().as_ref())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}
