mod chain;
mod instant;

pub use chain::*;
pub use instant::*;

use crate::errors::RustConversionError;
use wasm_bindgen::prelude::*;

use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;

#[wasm_bindgen(js_name=AssetLockProof)]
pub struct AssetLockProofWasm(AssetLockProof);

impl From<AssetLockProof> for AssetLockProofWasm {
    fn from(v: AssetLockProof) -> Self {
        AssetLockProofWasm(v)
    }
}

#[wasm_bindgen(js_name=createAssetLockProofInstance)]
pub fn create_asset_lock_proof_instance(raw_parameters: JsValue) -> Result<JsValue, JsValue> {
    let lock_type = js_sys::Reflect::get(&raw_parameters, &JsValue::from_str("type"))
        .map_err(|_| {
            RustConversionError::Error(String::from("error getting type from raw asset lock"))
                .to_js_value()
        })?
        .as_f64()
        .ok_or_else(|| JsValue::from_str("asset lock type must be a number"))?
        as u8;

    match lock_type {
        0 => InstantAssetLockProofWasm::new(raw_parameters).map(|v| v.into()),
        1 => ChainAssetLockProofWasm::new(raw_parameters).map(|v| v.into()),
        _ => Err(
            RustConversionError::Error(String::from("unrecognized asset lock type")).to_js_value(),
        ),
    }
}
