use crate::identifier::IdentifierWrapper;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityInsufficientBalanceError)]
pub struct IdentityInsufficientBalanceErrorWasm {
    inner: IdentityInsufficientBalanceError,
}

impl From<&IdentityInsufficientBalanceError> for IdentityInsufficientBalanceErrorWasm {
    fn from(e: &IdentityInsufficientBalanceError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityInsufficientBalanceError)]
impl IdentityInsufficientBalanceErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn identity_id(&self) -> IdentifierWrapper {
        (*self.inner.identity_id()).into()
    }

    #[wasm_bindgen(js_name=getBalance)]
    pub fn balance(&self) -> u32 {
        self.inner.balance() as u32
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
