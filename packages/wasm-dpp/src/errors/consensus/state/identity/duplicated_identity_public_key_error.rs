use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyError;
use dpp::consensus::ConsensusError;
use dpp::StateError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyStateError)]
pub struct DuplicatedIdentityPublicKeyStateErrorWasm {
    inner: DuplicatedIdentityPublicKeyError,
}

impl From<&DuplicatedIdentityPublicKeyError> for DuplicatedIdentityPublicKeyStateErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyStateError)]
impl DuplicatedIdentityPublicKeyStateErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedPublicKeysIds)]
    pub fn duplicated_public_keys_ids(&self) -> Vec<u32> {
        // TODO: key ids probably should be u32
        self.inner
            .duplicated_public_keys_ids()
            .iter()
            .map(|id| *id as u32)
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(StateError::DuplicatedIdentityPublicKeyError {
            duplicated_public_key_ids: vec![],
        })
        .code()
    }
}
