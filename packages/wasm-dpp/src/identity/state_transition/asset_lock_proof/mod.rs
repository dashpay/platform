mod chain;
mod instant;

pub use chain::*;
pub use instant::*;

use crate::errors::RustConversionError;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;
use crate::Deserialize;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;

#[derive(Deserialize, Clone)]
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

impl From<InstantAssetLockProofWasm> for AssetLockProofWasm {
    fn from(f: InstantAssetLockProofWasm) -> Self {
        AssetLockProof::Instant(f.into()).into()
    }
}

impl From<ChainAssetLockProofWasm> for AssetLockProofWasm {
    fn from(f: ChainAssetLockProofWasm) -> Self {
        AssetLockProof::Chain(f.into()).into()
    }
}

pub trait AssetLockProofLike {
    fn to_object(&self) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(js_class = AssetLockProof)]
impl AssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_asset_lock_proof: JsValue) -> Result<AssetLockProofWasm, JsValue> {
        let lock_type = get_lock_type(&raw_asset_lock_proof)?;

        match lock_type {
            0 => Ok(InstantAssetLockProofWasm::new(raw_asset_lock_proof)?.into()),
            1 => Ok(ChainAssetLockProofWasm::new(raw_asset_lock_proof)?.into()),
            _ => Err(
                RustConversionError::Error(String::from("unrecognized asset lock type"))
                    .to_js_value(),
            ),
        }
    }

    #[wasm_bindgen(js_name=createIdentifier)]
    pub fn create_identifier(&self) -> Result<IdentifierWrapper, JsValue> {
        match &self.0 {
            AssetLockProof::Instant(instant) => {
                InstantAssetLockProofWasm::from(instant.to_owned().clone()).create_identifier()
            }
            AssetLockProof::Chain(chain) => {
                ChainAssetLockProofWasm::from(chain.to_owned().clone()).create_identifier()
            }
        }
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        match &self.0 {
            AssetLockProof::Instant(instant) => {
                InstantAssetLockProofWasm::from(instant.to_owned().clone()).to_object()
            }
            AssetLockProof::Chain(chain) => {
                ChainAssetLockProofWasm::from(chain.to_owned().clone()).to_object()
            }
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
