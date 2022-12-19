use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractDecodeError)]
#[derive(Debug)]
pub struct DataContractDecodeError {
    // we have to store it as JsValue as the errors of 'class' Consensus are of different types
    errors: Vec<JsValue>,
    raw_data_contract: JsValue,
}

#[wasm_bindgen(js_class=DataContractDecodeError)]
impl DataContractDecodeError {
    #[wasm_bindgen(constructor)]
    pub fn new(errors: Vec<JsValue>, raw_data_contract: JsValue) -> Self {
        DataContractDecodeError {
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

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        let extended_message = if let Some(error_message) = self
            .errors
            .first()
            .map(|e| Into::<js_sys::Error>::into(e.clone()).message())
        {
            let narrowed_message = if self.errors.len() > 1 {
                format!(" and {} more", self.errors.len() - 1)
            } else {
                "".to_owned()
            };
            format!(": \"{error_message}\"{narrowed_message}")
        } else {
            "".to_owned()
        };

        format!("Data contract decode error{extended_message}")
    }
}
