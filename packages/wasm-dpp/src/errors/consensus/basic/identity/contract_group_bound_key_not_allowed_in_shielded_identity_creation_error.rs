use dpp::consensus::basic::identity::ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError)]
pub struct ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationErrorWasm {
    inner: ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError,
}

impl From<&ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError>
    for ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationErrorWasm
{
    fn from(e: &ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError)]
impl ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationErrorWasm {
    #[wasm_bindgen(js_name=getKeyId)]
    pub fn get_key_id(&self) -> u32 {
        self.inner.key_id()
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
