// Element type is not exposed through drive's verify feature
// We'll work with the serialized format instead
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyElementsResult {
    root_hash: Vec<u8>,
    elements: JsValue,
}

#[wasm_bindgen]
impl VerifyElementsResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn elements(&self) -> JsValue {
        self.elements.clone()
    }
}

#[wasm_bindgen(js_name = "verifyElements")]
pub fn verify_elements(
    _proof: &Uint8Array,
    _path: &Array,
    _keys: &Array,
    _platform_version_number: u32,
) -> Result<VerifyElementsResult, JsValue> {
    // This function requires Element type from grovedb which is not available in wasm context
    // TODO: Implement a version that works with serialized elements
    Err(JsValue::from_str(
        "verify_elements is not yet implemented for WASM due to grovedb dependency",
    ))
}
