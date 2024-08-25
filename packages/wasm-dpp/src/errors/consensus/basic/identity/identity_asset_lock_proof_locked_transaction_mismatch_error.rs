use dpp::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityAssetLockProofLockedTransactionMismatchError)]
pub struct IdentityAssetLockProofLockedTransactionMismatchErrorWasm {
    inner: IdentityAssetLockProofLockedTransactionMismatchError,
}

impl From<&IdentityAssetLockProofLockedTransactionMismatchError>
    for IdentityAssetLockProofLockedTransactionMismatchErrorWasm
{
    fn from(e: &IdentityAssetLockProofLockedTransactionMismatchError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockProofLockedTransactionMismatchError)]
impl IdentityAssetLockProofLockedTransactionMismatchErrorWasm {
    #[wasm_bindgen(js_name=getInstantLockTransactionId)]
    pub fn instant_lock_transaction_id(&self) -> Buffer {
        let tx_id = self.inner.instant_lock_transaction_id();
        Buffer::from_bytes(&tx_id.to_byte_array())
    }

    #[wasm_bindgen(js_name=getAssetLockTransactionId)]
    pub fn asset_lock_transaction_id(&self) -> Buffer {
        let tx_id = self.inner.asset_lock_transaction_id();
        let mut hash_bytes = tx_id.to_byte_array();
        hash_bytes.reverse();

        Buffer::from_bytes(&hash_bytes)
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
