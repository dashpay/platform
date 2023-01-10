use dpp::identity::errors::UnknownAssetLockProofTypeError;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen(js_name=UnknownAssetLockProofTypeError)]
pub struct UnknownAssetLockProofTypeErrorWasm(UnknownAssetLockProofTypeError);

impl From<&UnknownAssetLockProofTypeError> for UnknownAssetLockProofTypeErrorWasm {
    fn from(e: &UnknownAssetLockProofTypeError) -> Self {
        Self(e.clone())
    }
}

impl From<UnknownAssetLockProofTypeError> for UnknownAssetLockProofTypeErrorWasm {
    fn from(e: UnknownAssetLockProofTypeError) -> Self {
        Self(e)
    }
}

#[wasm_bindgen(js_class=UnknownAssetLockProofTypeError)]
impl UnknownAssetLockProofTypeErrorWasm {
    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> Option<u8> {
        self.0.asset_lock_type()
    }
}
