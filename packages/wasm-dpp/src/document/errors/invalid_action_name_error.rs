use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document action submitted")]
pub struct InvalidActionNameError {
    actions: Vec<String>,
}

#[wasm_bindgen]
impl InvalidActionNameError {
    #[wasm_bindgen(constructor)]
    pub fn new(actions: Vec<JsValue>) -> Self {
        let actions: Vec<String> = from_vec_js(&actions);
        Self { actions }
    }

    #[wasm_bindgen(js_name=getActions)]
    pub fn get_actions(&self) -> Vec<JsValue> {
        to_vec_js(self.actions.clone())
    }
}
