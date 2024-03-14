use crate::buffer::Buffer;

use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::data_trigger::data_trigger_invalid_result_error::DataTriggerInvalidResultError;
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataTriggerInvalidResultError)]
pub struct DataTriggerInvalidResultErrorWasm {
    inner: DataTriggerInvalidResultError,
}

impl From<&DataTriggerInvalidResultError> for DataTriggerInvalidResultErrorWasm {
    fn from(e: &DataTriggerInvalidResultError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataTriggerInvalidResultError)]
impl DataTriggerInvalidResultErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.data_contract_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
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

#[wasm_bindgen(js_name=DataTriggerActionInvalidResultError)]
pub struct DataTriggerActionInvalidResultErrorWasm {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    owner_id: Option<Identifier>,
    code: u32,
}

#[wasm_bindgen(js_class=DataTriggerActionInvalidResultError)]
impl DataTriggerActionInvalidResultErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getDocumentTransitionId)]
    pub fn document_transition_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_transition_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn owner_id(&self) -> Option<Buffer> {
        let owner_id = self.owner_id.as_ref()?;
        Some(Buffer::from_bytes(owner_id.as_bytes()))
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DataTriggerActionInvalidResultErrorWasm {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        owner_id: Option<Identifier>,
        code: u32,
    ) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            owner_id,
            code,
        }
    }
}
