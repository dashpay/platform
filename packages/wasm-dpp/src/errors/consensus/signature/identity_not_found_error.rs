use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::identifier::IdentifierWrapper;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
#[wasm_bindgen(js_name=IdentityNotFoundError)]
pub struct IdentityNotFoundErrorWasm {
    inner: IdentityNotFoundError,
}

impl From<&IdentityNotFoundError> for IdentityNotFoundErrorWasm {
    fn from(e: &IdentityNotFoundError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityNotFoundError)]
impl IdentityNotFoundErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(identity_id: IdentifierWrapper) -> Self {
        Self {
            inner: IdentityNotFoundError::new(identity_id.into()),
        }
    }

    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn get_identity_id(&self) -> IdentifierWrapper {
        self.inner.identity_id().into()
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
            .serialize_to_bytes_with_platform_version(PlatformVersion::first())
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
