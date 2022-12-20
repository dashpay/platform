use crate::buffer::Buffer;
use crate::document::state_transition::document_batch_transition::document_transition::from_document_transition_to_js_value;

use dpp::identifier::Identifier;
use dpp::prelude::DocumentTransition;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataTriggerConditionError)]
pub struct DataTriggerConditionErrorWasm {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,
    document_transition: Option<DocumentTransition>,
    owner_id: Option<Identifier>,
    code: u32,
}

#[wasm_bindgen(js_class=DataTriggerConditionError)]
impl DataTriggerConditionErrorWasm {
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

    #[wasm_bindgen(js_name=getTimestamp)]
    pub fn document_transition(&self) -> JsValue {
        if let Some(document_transition) = &self.document_transition {
            from_document_transition_to_js_value(document_transition.clone())
        } else {
            JsValue::undefined()
        }
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

impl DataTriggerConditionErrorWasm {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
        code: u32,
    ) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,
            document_transition,
            owner_id,
            code,
        }
    }
}
