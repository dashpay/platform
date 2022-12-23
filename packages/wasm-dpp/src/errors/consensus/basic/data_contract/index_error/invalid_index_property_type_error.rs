use dpp::util::json_schema::Index;
use wasm_bindgen::prelude::*;

use crate::IndexDefinitionWasm;

#[wasm_bindgen(js_name=InvalidIndexPropertyTypeError)]
pub struct InvalidIndexPropertyTypeErrorWasm {
    document_type: String,
    index_definition: Index,
    property_name: String,
    property_type: String,
    code: u32,
}

impl InvalidIndexPropertyTypeErrorWasm {
    pub fn new(
        document_type: String,
        index_definition: Index,
        property_name: String,
        property_type: String,
        code: u32,
    ) -> Self {
        InvalidIndexPropertyTypeErrorWasm {
            document_type,
            index_definition,
            property_name,
            property_type,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidIndexPropertyTypeError)]
impl InvalidIndexPropertyTypeErrorWasm {
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

    #[wasm_bindgen(js_name=getPropertyType)]
    pub fn get_property_type(&self) -> String {
        self.property_type.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
