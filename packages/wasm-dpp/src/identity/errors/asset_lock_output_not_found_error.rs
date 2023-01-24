use dpp::identity::errors::AssetLockOutputNotFoundError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=AssetLockOutputNotFoundError)]
pub struct AssetLockOutputNotFoundErrorWasm(AssetLockOutputNotFoundError);

impl From<&AssetLockOutputNotFoundError> for AssetLockOutputNotFoundErrorWasm {
    fn from(e: &AssetLockOutputNotFoundError) -> Self {
        Self(e.clone())
    }
}
