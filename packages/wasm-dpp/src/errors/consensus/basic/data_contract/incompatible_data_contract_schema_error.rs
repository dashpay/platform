use crate::buffer::Buffer;
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IncompatibleDataContractSchemaError)]
pub struct IncompatibleDataContractSchemaErrorWasm {
    data_contract_id: Identifier,
    operation: String,
    field_path: String,
    code: u32,
}

impl IncompatibleDataContractSchemaErrorWasm {
    pub fn new(
        data_contract_id: Identifier,
        operation: String,
        field_path: String,
        code: u32,
    ) -> Self {
        IncompatibleDataContractSchemaErrorWasm {
            data_contract_id,
            operation,
            field_path,
            code,
        }
    }
}

#[wasm_bindgen(js_class=IncompatibleDataContractSchemaError)]
impl IncompatibleDataContractSchemaErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

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
