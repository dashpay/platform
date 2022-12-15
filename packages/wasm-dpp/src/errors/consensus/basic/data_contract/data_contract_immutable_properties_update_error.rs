use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractImmutablePropertiesUpdateError)]
pub struct DataContractImmutablePropertiesUpdateErrorWasm {
    operation: String,
    field_path: String,
    code: u32,
}

impl DataContractImmutablePropertiesUpdateErrorWasm {
    pub fn new(operation: String, field_path: String, code: u32) -> Self {
        DataContractImmutablePropertiesUpdateErrorWasm {
            operation,
            field_path,
            code,
        }
    }
}

#[wasm_bindgen(js_class=DataContractImmutablePropertiesUpdateError)]
impl DataContractImmutablePropertiesUpdateErrorWasm {
    #[wasm_bindgen(js_name=getOperation)]
    pub fn get_operation(&self) -> String {
        self.operation.clone()
    }

    #[wasm_bindgen(js_name=getFieldPath)]
    pub fn get_field_path(&self) -> String {
        self.field_path.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
