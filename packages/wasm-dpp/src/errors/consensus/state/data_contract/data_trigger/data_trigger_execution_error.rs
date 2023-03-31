use crate::buffer::Buffer;
use crate::document::state_transition::document_batch_transition::document_transition::from_document_transition_to_js_value;

use dpp::identifier::Identifier;
use dpp::prelude::DocumentTransition;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataTriggerExecutionError)]
pub struct DataTriggerExecutionErrorWasm {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,
    code: u32,
}

#[wasm_bindgen(js_class=DataTriggerExecutionError)]
impl DataTriggerExecutionErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getExecutionError)]
    pub fn data_execution_error(&self) -> JsError {
        self.execution_error.clone()
    }

    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DataTriggerExecutionErrorWasm {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        code: u32,
    ) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,
            code,
        }
    }
}
