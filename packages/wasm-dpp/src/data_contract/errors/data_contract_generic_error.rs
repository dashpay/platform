use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractGenericError)]
#[derive(Debug)]
pub struct DataContractGenericError {
    message: String,
}

impl DataContractGenericError {
    pub fn new(message: String) -> Self {
        DataContractGenericError { message }
    }
}

#[wasm_bindgen(js_class=DataContractGenericError)]
impl DataContractGenericError {
    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.message.clone()
    }
}
