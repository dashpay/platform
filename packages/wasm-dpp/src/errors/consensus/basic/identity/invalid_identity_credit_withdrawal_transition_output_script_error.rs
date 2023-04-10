use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionOutputScriptError;
use dpp::consensus::codes::ErrorWithCode;
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
    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name=serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize()
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
