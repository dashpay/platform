use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointAlreadyConsumedError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityAssetLockTransactionOutPointAlreadyExistsError)]
pub struct IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm {
    inner: IdentityAssetLockTransactionOutPointAlreadyConsumedError,
}

impl From<&IdentityAssetLockTransactionOutPointAlreadyConsumedError>
    for IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm
{
    fn from(e: &IdentityAssetLockTransactionOutPointAlreadyConsumedError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockTransactionOutPointAlreadyExistsError)]
impl IdentityAssetLockTransactionOutPointAlreadyExistsErrorWasm {
    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn output_index(&self) -> usize {
        self.inner.output_index()
    }

    #[wasm_bindgen(js_name=getTransactionId)]
    pub fn transaction_id(&self) -> Buffer {
        let tx_id = self.inner.transaction_id();
        let mut tx_id_bytes = tx_id.to_byte_array();
        tx_id_bytes.reverse();
        Buffer::from_bytes(&tx_id_bytes)
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
