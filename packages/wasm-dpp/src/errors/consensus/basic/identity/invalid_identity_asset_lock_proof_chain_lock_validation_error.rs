use dpp::consensus::basic::identity::InvalidIdentityAssetLockProofChainLockValidationError;

use crate::buffer::Buffer;

use dpp::dashcore::hashes::Hash;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityAssetLockProofChainLockValidationErrorWasm)]
pub struct InvalidIdentityAssetLockProofChainLockValidationErrorWasm {
    inner: InvalidIdentityAssetLockProofChainLockValidationError,
}

impl From<&InvalidIdentityAssetLockProofChainLockValidationError>
    for InvalidIdentityAssetLockProofChainLockValidationErrorWasm
{
    fn from(e: &InvalidIdentityAssetLockProofChainLockValidationError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityAssetLockProofChainLockValidationErrorWasm)]
impl InvalidIdentityAssetLockProofChainLockValidationErrorWasm {
    #[wasm_bindgen(js_name=getTransactionId)]
    pub fn transaction_id(&self) -> Buffer {
        let tx_id = self.inner.transaction_id();
        let mut tx_id_bytes = tx_id.to_byte_array();
        tx_id_bytes.reverse();
        Buffer::from_bytes(&tx_id_bytes)
    }

    #[wasm_bindgen(js_name=getHeightReportedNotLocked)]
    pub fn get_height_reported_not_locked(&self) -> u32 {
        self.inner.height_reported_not_locked()
    }
}
