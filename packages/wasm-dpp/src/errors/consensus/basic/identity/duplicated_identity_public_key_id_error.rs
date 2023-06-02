use crate::buffer::Buffer;
use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdBasicError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::serialization_traits::PlatformSerializable;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyIdError)]
pub struct DuplicatedIdentityPublicKeyIdErrorWasm {
    inner: DuplicatedIdentityPublicKeyIdBasicError,
}

impl From<&DuplicatedIdentityPublicKeyIdBasicError> for DuplicatedIdentityPublicKeyIdErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyIdBasicError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyIdError)]
impl DuplicatedIdentityPublicKeyIdErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn duplicated_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.inner
            .duplicated_ids()
            .iter()
            .map(|id| JsValue::from(*id))
            .collect()
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
