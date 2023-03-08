use dpp::document::document_transition::document_base_transition::JsonValue;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDataContractIdError)]
pub struct MissingDataContractIdErrorWasm {
    raw_document_transition: JsonValue,
    code: u32,
}

impl MissingDataContractIdErrorWasm {
    pub fn new(raw_document_transition: JsonValue, code: u32) -> Self {
        MissingDataContractIdErrorWasm {
            raw_document_transition,
            code,
        }
    }
}

#[wasm_bindgen(js_class=MissingDataContractIdError)]
impl MissingDataContractIdErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }

    #[wasm_bindgen(js_name=rawDocumentTransition)]
    pub fn raw_document_transition(&self) -> wasm_bindgen::JsValue {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        self.raw_document_transition
            .serialize(&serializer)
            .expect("implements Serialize")
    }
}
