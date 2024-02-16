use crate::buffer::Buffer;

use dpp::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityContractNonceError;
use dpp::consensus::ConsensusError;
use dpp::identity::identity_contract_nonce::MergeIdentityContractNonceResult;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityContractNonceError)]
pub struct InvalidIdentityContractNonceErrorWasm {
    inner: InvalidIdentityContractNonceError,
}

impl From<&InvalidIdentityContractNonceError> for InvalidIdentityContractNonceErrorWasm {
    fn from(e: &InvalidIdentityContractNonceError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityContractNonceError)]
impl InvalidIdentityContractNonceErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn identity_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.identity_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getCurrentIdentityContractNonce)]
    pub fn current_identity_contract_nonce(&self) -> Option<u64> {
        self.inner
            .current_identity_contract_nonce()
            .map(|nonce| *nonce as u64)
    }

    #[wasm_bindgen(js_name=getSettingIdentityContractNonce)]
    pub fn setting_identity_contract_nonce(&self) -> u64 {
        *self.inner.setting_identity_contract_nonce() as u64
    }

    #[wasm_bindgen(js_name=getError)]
    pub fn error(&self) -> js_sys::Error {
        match self.inner.error() {
            MergeIdentityContractNonceResult::NonceTooFarInFuture => {
                js_sys::Error::new("nonce too far in future")
            }
            MergeIdentityContractNonceResult::NonceTooFarInPast => {
                js_sys::Error::new("nonce too far in past")
            }
            MergeIdentityContractNonceResult::NonceAlreadyPresentAtTip => {
                js_sys::Error::new("nonce already present at tip")
            }
            MergeIdentityContractNonceResult::NonceAlreadyPresentInPast(nonce) => {
                js_sys::Error::new(&format!("nonce already present in past: {}", nonce))
            }
            MergeIdentityContractNonceResult::MergeIdentityContractNonceSuccess(_) => {
                js_sys::Error::new("no error")
            }
        }
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}
