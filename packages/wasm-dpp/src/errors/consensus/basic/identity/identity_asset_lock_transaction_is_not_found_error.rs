use dpp::consensus::basic::identity::IdentityAssetLockTransactionIsNotFoundError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityAssetLockTransactionIsNotFoundError)]
pub struct IdentityAssetLockTransactionIsNotFoundErrorWasm {
    inner: IdentityAssetLockTransactionIsNotFoundError,
}

impl From<&IdentityAssetLockTransactionIsNotFoundError>
    for IdentityAssetLockTransactionIsNotFoundErrorWasm
{
    fn from(e: &IdentityAssetLockTransactionIsNotFoundError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityAssetLockTransactionIsNotFoundError)]
impl IdentityAssetLockTransactionIsNotFoundErrorWasm {
    #[wasm_bindgen(js_name=getTransactionId)]
    pub fn transaction_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.transaction_id())
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
