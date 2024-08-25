use dpp::consensus::basic::identity::InvalidAssetLockProofCoreChainHeightError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidAssetLockProofCoreChainHeightError)]
pub struct InvalidAssetLockProofCoreChainHeightErrorWasm {
    inner: InvalidAssetLockProofCoreChainHeightError,
}

impl From<&InvalidAssetLockProofCoreChainHeightError>
    for InvalidAssetLockProofCoreChainHeightErrorWasm
{
    fn from(e: &InvalidAssetLockProofCoreChainHeightError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidAssetLockProofCoreChainHeightError)]
impl InvalidAssetLockProofCoreChainHeightErrorWasm {
    #[wasm_bindgen(js_name=getProofCoreChainLockedHeight)]
    pub fn proof_core_chain_locked_height(&self) -> u32 {
        self.inner.proof_core_chain_locked_height()
    }

    #[wasm_bindgen(js_name=getCurrentCoreChainLockedHeight)]
    pub fn current_core_chain_locked_height(&self) -> u32 {
        self.inner.current_core_chain_locked_height()
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
