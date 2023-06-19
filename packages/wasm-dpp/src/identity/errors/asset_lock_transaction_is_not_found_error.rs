use dpp::identity::errors::AssetLockTransactionIsNotFoundError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=AssetLockTransactionIsNotFoundError)]
pub struct AssetLockTransactionIsNotFoundErrorWasm(AssetLockTransactionIsNotFoundError);

impl From<&AssetLockTransactionIsNotFoundError> for AssetLockTransactionIsNotFoundErrorWasm {
    fn from(e: &AssetLockTransactionIsNotFoundError) -> Self {
        Self(e.clone())
    }
}

#[wasm_bindgen(js_class=AssetLockTransactionIsNotFoundError)]
impl AssetLockTransactionIsNotFoundErrorWasm {
    #[wasm_bindgen(js_name=getTransactionId)]
    pub fn transaction_id(&self) -> JsValue {
        hex::encode(self.0.transaction_id()).into()
    }
}
