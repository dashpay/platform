use dpp::dashcore;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Transaction)]
#[derive(Debug, Clone)]
pub struct WasmTx(dashcore::Transaction);

#[wasm_bindgen(js_class = Transaction)]
impl WasmTx {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmTx {
        Self(dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
        })
    }

    pub fn version(&self) -> i32 {
        self.0.version
    }
}
