use dpp::state_transition::fee::Credits;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=BalanceIsNotEnoughError)]
pub struct BalanceIsNotEnoughErrorWasm {
    balance: Credits,
    fee: Credits,
    code: u32,
}

impl BalanceIsNotEnoughErrorWasm {
    pub fn new(balance: Credits, fee: Credits, code: u32) -> Self {
        BalanceIsNotEnoughErrorWasm { balance, fee, code }
    }
}

#[wasm_bindgen(js_class=BalanceIsNotEnoughError)]
impl BalanceIsNotEnoughErrorWasm {
    #[wasm_bindgen(js_name=getBalance)]
    pub fn get_balance(&self) -> f64 {
        self.balance as f64
    }

    #[wasm_bindgen(js_name=getFee)]
    pub fn get_fee(&self) -> f64 {
        self.fee as f64
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
