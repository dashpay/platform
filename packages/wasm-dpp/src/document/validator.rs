use std::sync::Arc;

use dpp::document::document_validator::DocumentValidator;

use wasm_bindgen::prelude::*;

use crate::version::ProtocolVersionValidatorWasm;

#[wasm_bindgen(js_name = DocumentValidator)]
pub struct DocumentValidatorWasm(DocumentValidator);

#[wasm_bindgen(js_class=DocumentValidator)]
impl DocumentValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(protocol_validator: ProtocolVersionValidatorWasm) -> DocumentValidatorWasm {
        DocumentValidatorWasm(DocumentValidator::new(Arc::new(protocol_validator.into())))
    }
}

impl From<DocumentValidator> for DocumentValidatorWasm {
    fn from(doc_validator: DocumentValidator) -> Self {
        DocumentValidatorWasm(doc_validator)
    }
}

impl From<DocumentValidatorWasm> for DocumentValidator {
    fn from(val: DocumentValidatorWasm) -> Self {
        val.0
    }
}
