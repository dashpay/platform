use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use crate::buffer::Buffer;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointNotEnoughBalanceError;
use dpp::dashcore::hashes::Hash;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityAssetLockTransactionOutPointNotEnoughBalanceError)]
pub struct IdentityAssetLockTransactionOutPointNotEnoughBalanceErrorWasm {
    inner: IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
}

impl From<&IdentityAssetLockTransactionOutPointNotEnoughBalanceError>
    for IdentityAssetLockTransactionOutPointNotEnoughBalanceErrorWasm
{
    fn from(e: &IdentityAssetLockTransactionOutPointNotEnoughBalanceError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockTransactionOutPointNotEnoughBalanceError)]
impl IdentityAssetLockTransactionOutPointNotEnoughBalanceErrorWasm {
    #[wasm_bindgen(js_name=getTransactionId)]
    pub fn transaction_id(&self) -> Buffer {
        let tx_id = self.inner.transaction_id();
        let mut tx_id_bytes = tx_id.to_byte_array();
        tx_id_bytes.reverse();
        Buffer::from_bytes(&tx_id_bytes)
    }

    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn output_index(&self) -> usize {
        self.inner.output_index()
    }

    #[wasm_bindgen(js_name=getInitialAssetLockCredits)]
    pub fn initial_asset_lock_credits(&self) -> u64 {
        self.inner.initial_asset_lock_credits()
    }

    #[wasm_bindgen(js_name=getCreditsLeft)]
    pub fn credits_left(&self) -> u64 {
        self.inner.credits_left()
    }

    #[wasm_bindgen(js_name=getCreditsRequired)]
    pub fn credits_required(&self) -> u64 {
        self.inner.credits_required()
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
