use crate::buffer::Buffer;

use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataTriggerConditionError)]
pub struct DataTriggerConditionErrorWasm {
    inner: DataTriggerConditionError,
}

impl From<&DataTriggerConditionError> for DataTriggerConditionErrorWasm {
    fn from(e: &DataTriggerConditionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataTriggerConditionError)]
impl DataTriggerConditionErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.data_contract_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.inner.message().to_string()
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

#[wasm_bindgen(js_name=DataTriggerActionConditionError)]
pub struct DataTriggerActionConditionErrorWasm {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,
    owner_id: Option<Identifier>,
    code: u32,
}

#[wasm_bindgen(js_class=DataTriggerActionConditionError)]
impl DataTriggerActionConditionErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getDocumentTransitionId)]
    pub fn document_transition_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_transition_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn message(&self) -> String {
        self.message.clone()
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

impl DataTriggerActionConditionErrorWasm {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        owner_id: Option<Identifier>,
        code: u32,
    ) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,
            owner_id,
            code,
        }
    }
}
