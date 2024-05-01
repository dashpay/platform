use dpp::consensus::basic::identity::IdentityCreditTransferToSelfError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityCreditTransferToSelfError)]
pub struct IdentityCreditTransferToSelfErrorWasm {
    inner: IdentityCreditTransferToSelfError,
}

impl From<&IdentityCreditTransferToSelfError> for IdentityCreditTransferToSelfErrorWasm {
    fn from(e: &IdentityCreditTransferToSelfError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityCreditTransferToSelfError)]
impl IdentityCreditTransferToSelfErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
