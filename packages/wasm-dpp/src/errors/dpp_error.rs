use crate::identity::errors::{
    AssetLockOutputNotFoundErrorWasm, AssetLockTransactionIsNotFoundErrorWasm,
};
use dpp::DPPError;
use wasm_bindgen::JsValue;

pub fn from_dpp_error_ref(e: &DPPError) -> JsValue {
    match e {
        DPPError::AssetLockOutputNotFoundError(e) => {
            AssetLockOutputNotFoundErrorWasm::from(e).into()
        }
        DPPError::AssetLockTransactionIsNotFoundError(e) => {
            AssetLockTransactionIsNotFoundErrorWasm::from(e).into()
        }
        _ => unimplemented!(),
    }
}
