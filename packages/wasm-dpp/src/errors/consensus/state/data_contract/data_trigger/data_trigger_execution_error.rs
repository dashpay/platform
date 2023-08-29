use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataTriggerExecutionError)]
pub struct DataTriggerExecutionErrorWasm {
    inner: DataTriggerExecutionError,
}

impl From<&DataTriggerExecutionError> for DataTriggerExecutionErrorWasm {
    fn from(e: &DataTriggerExecutionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataTriggerExecutionError)]
impl DataTriggerExecutionErrorWasm {
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
            .serialize_to_bytes()
            .map_err(JsError::from)?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}

#[wasm_bindgen(js_name=DataTriggerActionExecutionError)]
pub struct DataTriggerActionExecutionErrorWasm {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,
    execution_error: JsError,
    owner_id: Option<Identifier>,
    code: u32,
}

#[wasm_bindgen(js_class=DataTriggerActionExecutionError)]
impl DataTriggerActionExecutionErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getExecutionError)]
    pub fn data_execution_error(&self) -> JsError {
        self.execution_error.clone()
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

impl DataTriggerActionExecutionErrorWasm {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        execution_error: wasm_bindgen::JsError,
        owner_id: Option<Identifier>,
        code: u32,
    ) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,
            execution_error,
            owner_id,
            code,
        }
    }
}
