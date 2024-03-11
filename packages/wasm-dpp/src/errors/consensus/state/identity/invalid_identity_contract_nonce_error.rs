use crate::buffer::Buffer;

use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
use dpp::consensus::ConsensusError;
use dpp::identity::identity_nonce::MergeIdentityNonceResult;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityNonceError)]
pub struct InvalidIdentityNonceErrorWasm {
    inner: InvalidIdentityNonceError,
}

impl From<&InvalidIdentityNonceError> for InvalidIdentityNonceErrorWasm {
    fn from(e: &InvalidIdentityNonceError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityNonceError)]
impl InvalidIdentityNonceErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn identity_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.identity_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getCurrentIdentityContractNonce)]
    pub fn current_identity_contract_nonce(&self) -> Option<u64> {
        self.inner.current_identity_contract_nonce().copied()
    }

    #[wasm_bindgen(js_name=getSettingIdentityContractNonce)]
    pub fn setting_identity_contract_nonce(&self) -> u64 {
        *self.inner.setting_identity_contract_nonce() as u64
    }

    #[wasm_bindgen(js_name=getError)]
    pub fn error(&self) -> js_sys::Error {
        match self.inner.error() {
            MergeIdentityNonceResult::NonceTooFarInFuture => {
                js_sys::Error::new("nonce too far in future")
            }
            MergeIdentityNonceResult::NonceTooFarInPast => {
                js_sys::Error::new("nonce too far in past")
            }
            MergeIdentityNonceResult::NonceAlreadyPresentAtTip => {
                js_sys::Error::new("nonce already present at tip")
            }
            MergeIdentityNonceResult::NonceAlreadyPresentInPast(nonce) => {
                js_sys::Error::new(&format!("nonce already present in past: {}", nonce))
            }
            MergeIdentityNonceResult::MergeIdentityNonceSuccess(_) => {
                js_sys::Error::new("no error")
            }
            MergeIdentityNonceResult::InvalidNonce => js_sys::Error::new("invalid nonce"),
        }
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
