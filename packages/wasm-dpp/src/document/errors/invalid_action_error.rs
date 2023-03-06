use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid action: {:?}", action)]
pub struct InvalidActionError {
    // the point is how we hold all there different types in  the Vector
    action: JsValue,
}

#[wasm_bindgen(js_class=InvalidActiontError)]
impl InvalidActionError {
    #[wasm_bindgen(constructor)]
    pub fn new(action: JsValue) -> InvalidActionError {
        Self { action }
    }
}
