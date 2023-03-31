use dpp::consensus::basic::identity::InvalidIdentityAssetLockTransactionError;

use dpp::consensus::basic::BasicError;
use dpp::consensus::codes::ErrorWithCode;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityAssetLockTransactionError)]
pub struct InvalidIdentityAssetLockTransactionErrorWasm {
    error_message: String,
    code: u32,
}

#[wasm_bindgen(js_class=InvalidIdentityAssetLockTransactionError)]
impl InvalidIdentityAssetLockTransactionErrorWasm {
    pub fn new(error_message: String, code: u32) -> Self {
        Self {
            error_message,
            code,
        }
    }

    #[wasm_bindgen(js_name=getErrorMessage)]
    pub fn get_error_message(&self) -> String {
        self.error_message.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
