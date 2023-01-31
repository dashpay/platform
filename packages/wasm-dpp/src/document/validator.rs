use std::sync::Arc;

use dpp::{
    document::{document_validator::DocumentValidator, property_names, IDENTIFIER_FIELDS},
    prelude::DataContract,
    util::json_value::{JsonValueExt, ReplaceWith},
};

use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::{
    raw_document_from_js_value,
    utils::{ToSerdeJSONExt, WithJsError},
    validation::ValidationResultWasm,
    version::ProtocolVersionValidatorWasm,
    DataContractWasm,
};

#[wasm_bindgen(js_name = DocumentValidator)]
pub struct DocumentValidatorWasm(DocumentValidator);

#[wasm_bindgen(js_class=DocumentValidator)]
impl DocumentValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(protocol_validator: ProtocolVersionValidatorWasm) -> DocumentValidatorWasm {
        DocumentValidatorWasm(DocumentValidator::new(Arc::new(protocol_validator.into())))
    }

    pub fn validate(
        &self,
        js_raw_document: &JsValue,
        js_data_contract: &DataContractWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_document = raw_document_from_js_value(js_raw_document, js_data_contract.inner())?;
        let validation_result = self
            .0
            .validate(&raw_document, js_data_contract.inner())
            .with_js_error()?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
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
