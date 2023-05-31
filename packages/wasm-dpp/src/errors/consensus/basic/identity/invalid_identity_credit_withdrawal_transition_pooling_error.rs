use crate::buffer::Buffer;
use dpp::consensus::basic::identity::NotImplementedIdentityCreditWithdrawalTransitionPoolingError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::serialization_traits::PlatformSerializable;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=NotImplementedIdentityCreditWithdrawalTransitionPoolingError)]
pub struct NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm {
    inner: NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
}

impl From<&NotImplementedIdentityCreditWithdrawalTransitionPoolingError>
    for NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm
{
    fn from(e: &NotImplementedIdentityCreditWithdrawalTransitionPoolingError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=NotImplementedIdentityCreditWithdrawalTransitionPoolingError)]
impl NotImplementedIdentityCreditWithdrawalTransitionPoolingErrorWasm {
    #[wasm_bindgen(js_name=getPooling)]
    pub fn pooling(&self) -> u8 {
        self.inner.pooling()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
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
