use dpp::consensus::basic::identity::{DuplicatedIdentityPublicKeyIdError, IdentityAssetLockTransactionOutPointAlreadyExistsError};
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;
use dpp::dashcore::Txid;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityAssetLockTransactionOutPointAlreadyExistsError)]
pub struct IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm {
    inner: IdentityAssetLockTransactionOutPointAlreadyExistsError
}

impl From<&IdentityAssetLockTransactionOutPointAlreadyExistsError> for IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm {
    fn from(e: &IdentityAssetLockTransactionOutPointAlreadyExistsError) -> Self {
        Self {
            inner: e.clone()
        }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockTransactionOutPointAlreadyExistsError)]
impl IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn output_index(&self) -> usize {
        self.inner.output_index()
    }

    #[wasm_bindgen(js_name=getTransactionId)]
    pub fn transaction_id(&self) -> Buffer {
        let tx_id = self.inner.transaction_id();
        Buffer::from_bytes(tx_id.to_vec().as_ref())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}