use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Buffer;

    #[wasm_bindgen(constructor)]
    pub fn new() -> Buffer;

    #[wasm_bindgen(constructor, js_name = "from")]
    pub fn from_bytes(js_sys: &[u8]) -> Buffer;

    #[wasm_bindgen(constructor, js_name = "from")]
    pub fn from_string(js_sys: String) -> Buffer;
}
