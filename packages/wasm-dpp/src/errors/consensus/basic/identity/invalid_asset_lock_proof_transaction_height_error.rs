use crate::buffer::Buffer;
use dpp::consensus::basic::identity::InvalidAssetLockProofTransactionHeightError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::serialization_traits::PlatformSerializable;
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

    #[wasm_bindgen(js_name=getTransactionHeight)]
    pub fn transaction_height(&self) -> Option<u32> {
        self.inner.transaction_height()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name=serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize()
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
