use dpp::util::json_schema::Index;
use wasm_bindgen::prelude::*;

use crate::IndexDefinitionWasm;

#[wasm_bindgen(js_name=InvalidIndexedPropertyConstraintError)]
pub struct InvalidIndexedPropertyConstraintErrorWasm {
    document_type: String,
    index_definition: Index,
    property_name: String,
    constraint_name: String,
    reason: String,
    code: u32,
}

impl InvalidIndexedPropertyConstraintErrorWasm {
    pub fn new(
        document_type: String,
        index_definition: Index,
        property_name: String,
        constraint_name: String,
        reason: String,
        code: u32,
    ) -> Self {
        InvalidIndexedPropertyConstraintErrorWasm {
            document_type,
            index_definition,
            property_name,
            constraint_name,
            reason,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidIndexedPropertyConstraintError)]
impl InvalidIndexedPropertyConstraintErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.document_type.clone()
    }

    #[wasm_bindgen(js_name=getIndexDefinition)]
    pub fn get_index_definition(&self) -> JsValue {
        IndexDefinitionWasm::from(self.index_definition.clone()).into()
    }

    #[wasm_bindgen(js_name=getPropertyName)]
    pub fn get_property_name(&self) -> String {
        self.property_name.clone()
    }

    #[wasm_bindgen(js_name=getConstraintName)]
    pub fn get_constraint_name(&self) -> String {
        self.constraint_name.clone()
    }

    #[wasm_bindgen(js_name=getReason)]
    pub fn get_reason(&self) -> String {
        self.reason.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
