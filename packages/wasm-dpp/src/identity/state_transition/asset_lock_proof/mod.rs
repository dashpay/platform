mod chain;
mod instant;

pub use chain::*;
pub use instant::*;
use serde::Serialize;

use crate::{errors::RustConversionError, with_js_error};
use wasm_bindgen::prelude::*;

use crate::Deserialize;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;

#[derive(Deserialize)]
#[wasm_bindgen(js_name=AssetLockProof)]
pub struct AssetLockProofWasm(AssetLockProof);

impl From<AssetLockProof> for AssetLockProofWasm {
    fn from(v: AssetLockProof) -> Self {
        AssetLockProofWasm(v)
    }
}

impl From<AssetLockProofWasm> for AssetLockProof {
    fn from(v: AssetLockProofWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = AssetLockProof)]
impl AssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_asset_lock_proof: JsValue) -> Result<AssetLockProofWasm, JsValue> {
        let lock_type = get_lock_type(&raw_asset_lock_proof)?;

        match lock_type {
            0 => Ok(Self::from(AssetLockProof::Instant(
                InstantAssetLockProofWasm::new(raw_asset_lock_proof)?.into(),
            ))),
            1 => Ok(Self::from(AssetLockProof::Chain(
                ChainAssetLockProofWasm::new(raw_asset_lock_proof)?.into(),
            ))),
            _ => Err(
                RustConversionError::Error(String::from("unrecognized asset lock type"))
                    .to_js_value(),
            ),
        }
    }
}

fn get_lock_type(raw_asset_lock_proof: &JsValue) -> Result<u8, JsValue> {
    let lock_type = js_sys::Reflect::get(&raw_asset_lock_proof, &JsValue::from_str("type"))
        .map_err(|_| {
            RustConversionError::Error(String::from("error getting type from raw asset lock"))
                .to_js_value()
        })?
        .as_f64()
        .ok_or_else(|| JsValue::from_str("asset lock type must be a number"))?
        as u8;

    Ok(lock_type)
}

#[wasm_bindgen(js_name=createAssetLockProofInstance)]
pub fn create_asset_lock_proof_instance(raw_parameters: JsValue) -> Result<JsValue, JsValue> {
    let lock_type = get_lock_type(&raw_parameters)?;

    match lock_type {
        0 => InstantAssetLockProofWasm::new(raw_parameters).map(|v| v.into()),
        1 => ChainAssetLockProofWasm::new(raw_parameters).map(|v| v.into()),
        _ => Err(
            RustConversionError::Error(String::from("unrecognized asset lock type")).to_js_value(),
        ),
    }
}
