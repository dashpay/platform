use dpp::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
use dpp::consensus::ConsensusError;
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
        Buffer::from_bytes(tx_id.to_vec().as_ref())
    }

    #[wasm_bindgen(js_name=getAssetLockTransactionId)]
    pub fn asset_lock_transaction_id(&self) -> Buffer {
        let tx_id = self.inner.asset_lock_transaction_id();
        Buffer::from_bytes(tx_id.to_vec().as_ref())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}
