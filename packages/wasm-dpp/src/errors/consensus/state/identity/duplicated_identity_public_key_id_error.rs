use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdError;
use dpp::consensus::ConsensusError;
use dpp::StateError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyIdStateError)]
pub struct DuplicatedIdentityPublicKeyIdStateErrorWasm {
    inner: DuplicatedIdentityPublicKeyIdError,
}

impl From<&DuplicatedIdentityPublicKeyIdError> for DuplicatedIdentityPublicKeyIdStateErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyIdError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyIdStateError)]
impl DuplicatedIdentityPublicKeyIdStateErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn duplicated_ids(&self) -> Vec<u32> {
        // TODO: key ids probably should be u32
        self.inner
            .duplicated_ids()
            .iter()
            .map(|id| *id as u32)
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(StateError::DuplicatedIdentityPublicKeyIdError {
            duplicated_ids: vec![],
        })
        .code()
    }
}
