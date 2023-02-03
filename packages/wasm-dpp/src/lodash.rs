use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "lodash")]
extern "C" {
    #[wasm_bindgen(js_name = "set")]
    pub fn lodash_set(object: &JsValue, path: &str, value: JsValue);
}
