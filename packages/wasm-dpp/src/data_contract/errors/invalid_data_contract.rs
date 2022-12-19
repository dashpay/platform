use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDataContractError)]
#[derive(Debug)]
pub struct InvalidDataContractError {
    // we have to store it as JsValue as the errors of 'class' Consensus are of different types
    errors: Vec<JsValue>,
    raw_data_contract: JsValue,
}

#[wasm_bindgen(js_class=InvalidDataContractError)]
impl InvalidDataContractError {
    pub fn new(errors: Vec<JsValue>, raw_data_contract: JsValue) -> Self {
        InvalidDataContractError {
            errors,
            raw_data_contract,
        }
    }

    #[wasm_bindgen(js_name=getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.clone().into_iter().map(JsValue::from).collect()
    }

    #[wasm_bindgen(js_name=getRawDataContract)]
    pub fn get_raw_data_contract(&self) -> JsValue {
        self.raw_data_contract.clone()
    }
}
