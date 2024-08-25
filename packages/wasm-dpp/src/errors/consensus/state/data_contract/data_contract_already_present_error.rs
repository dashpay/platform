use crate::buffer::Buffer;
use crate::identifier::IdentifierWrapper;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::ConsensusError;

use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name=DataContractAlreadyPresentError)]
pub struct DataContractAlreadyPresentErrorWasm {
    inner: DataContractAlreadyPresentError,
}

impl From<&DataContractAlreadyPresentError> for DataContractAlreadyPresentErrorWasm {
    fn from(e: &DataContractAlreadyPresentError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractAlreadyPresentError)]
impl DataContractAlreadyPresentErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(data_contract_id: IdentifierWrapper) -> Self {
        Self {
            inner: DataContractAlreadyPresentError::new(data_contract_id.into()),
        }
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.data_contract_id().to_owned().into()
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
