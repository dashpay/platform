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
            special_transaction_payload: None
        })
    }

    pub fn version(&self) -> u16 {
        self.0.version
    }
}

impl Default for WasmTx {
    fn default() -> Self {
        Self::new()
    }
}
