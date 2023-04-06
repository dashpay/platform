use js_sys::ArrayBuffer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Buffer;

    #[wasm_bindgen(constructor)]
    pub fn new() -> Buffer;

    #[wasm_bindgen(static_method_of = Buffer, js_name = from)]
    pub fn from_bytes(js_sys: &[u8]) -> Buffer;

    #[wasm_bindgen(static_method_of = Buffer, js_name = from)]
    pub fn from_bytes_owned(js_sys: Vec<u8>) -> Buffer;

    #[wasm_bindgen(static_method_of = Buffer, js_name = from)]
    pub fn from_string(js_sys: String) -> Buffer;

    #[wasm_bindgen(method, getter)]
    pub fn buffer(this: &Buffer) -> ArrayBuffer;

    #[wasm_bindgen(method, getter, js_name = byteOffset)]
    pub fn byte_offset(this: &Buffer) -> u32;

    #[wasm_bindgen(method, getter)]
    pub fn length(this: &Buffer) -> u32;
}
