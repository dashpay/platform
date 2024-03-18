use dpp::consensus::basic::document::IdentityContractNonceOutOfBoundsError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityContractNonceOutOfBoundsError)]
pub struct IdentityContractNonceOutOfBoundsErrorWasm {
    inner: IdentityContractNonceOutOfBoundsError,
}

impl From<&IdentityContractNonceOutOfBoundsError> for IdentityContractNonceOutOfBoundsErrorWasm {
    fn from(e: &IdentityContractNonceOutOfBoundsError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityContractNonceOutOfBoundsError)]
impl IdentityContractNonceOutOfBoundsErrorWasm {
    #[wasm_bindgen(js_name=getIdentityContractNonce)]
    pub fn get_identity_contract_nonce(&self) -> u64 {
        self.inner.identity_contract_nonce()
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
