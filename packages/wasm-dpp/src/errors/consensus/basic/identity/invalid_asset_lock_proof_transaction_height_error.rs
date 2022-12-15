use dpp::consensus::basic::identity::InvalidAssetLockProofTransactionHeightError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidAssetLockProofTransactionHeightError)]
pub struct InvalidAssetLockProofTransactionHeightErrorWasm {
    inner: InvalidAssetLockProofTransactionHeightError,
}

impl From<&InvalidAssetLockProofTransactionHeightError>
    for InvalidAssetLockProofTransactionHeightErrorWasm
{
    fn from(e: &InvalidAssetLockProofTransactionHeightError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidAssetLockProofTransactionHeightError)]
impl InvalidAssetLockProofTransactionHeightErrorWasm {
    #[wasm_bindgen(js_name=getProofCoreChainLockedHeight)]
    pub fn proof_core_chain_locked_height(&self) -> u32 {
        self.inner.proof_core_chain_locked_height()
    }

    #[wasm_bindgen(js_name=getCurrentCoreChainLockedHeight)]
    pub fn current_core_chain_locked_height(&self) -> Option<u32> {
        self.inner.current_core_chain_locked_height()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }
}
