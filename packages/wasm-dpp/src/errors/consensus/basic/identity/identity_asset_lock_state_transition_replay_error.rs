use dpp::consensus::basic::identity::IdentityAssetLockStateTransitionReplayError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use dpp::dashcore::hashes::Hash;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityAssetLockTransactionReplayError)]
pub struct IdentityAssetLockStateTransitionReplayErrorWasm {
    inner: IdentityAssetLockStateTransitionReplayError,
}

impl From<&IdentityAssetLockStateTransitionReplayError>
    for IdentityAssetLockStateTransitionReplayErrorWasm
{
    fn from(e: &IdentityAssetLockStateTransitionReplayError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockTransactionReplayError)]
impl IdentityAssetLockStateTransitionReplayErrorWasm {
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
    #[wasm_bindgen(js_name=getStateTransitionId)]
    pub fn state_transition_id(&self) -> Buffer {
        let state_transition_id = self.inner.state_transition_id();
        let state_transition_id_bytes = state_transition_id.to_buffer();
        Buffer::from_bytes(&state_transition_id_bytes)
    }
    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn output_index(&self) -> usize {
        self.inner.output_index()
    }
}
