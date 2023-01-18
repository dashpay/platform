use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=BalanceIsNotEnoughError)]
pub struct BalanceIsNotEnoughErrorWasm {
    balance: u64,
    fee: i64,
    code: u32,
}

impl BalanceIsNotEnoughErrorWasm {
    pub fn new(balance: u64, fee: i64, code: u32) -> Self {
        BalanceIsNotEnoughErrorWasm { balance, fee, code }
    }
}

#[wasm_bindgen(js_class=BalanceIsNotEnoughError)]
impl BalanceIsNotEnoughErrorWasm {
    #[wasm_bindgen(js_name=getBalance)]
    pub fn get_balance(&self) -> u64 {
        self.balance
    }

    #[wasm_bindgen(js_name=getFee)]
    pub fn get_fee(&self) -> i64 {
        self.fee
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
