use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyIdError)]
pub struct DuplicatedIdentityPublicKeyIdErrorWasm {
    inner: DuplicatedIdentityPublicKeyIdError,
}

impl From<&DuplicatedIdentityPublicKeyIdError> for DuplicatedIdentityPublicKeyIdErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyIdError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyIdError)]
impl DuplicatedIdentityPublicKeyIdErrorWasm {
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
        ConsensusError::from(self.inner.clone()).code()
    }
}
