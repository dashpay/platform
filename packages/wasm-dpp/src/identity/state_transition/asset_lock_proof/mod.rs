mod chain;
mod instant;

pub use chain::*;
pub use instant::*;
use std::sync::Arc;

use crate::errors::{from_dpp_err, RustConversionError};
use dpp::dashcore::consensus;
use dpp::dashcore::hashes::hex::FromHex;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::identifier::IdentifierWrapper;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::validation::ValidationResultWasm;
use crate::{Deserialize, StateTransitionExecutionContextWasm};
use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockProof, AssetLockTransactionValidator,
};

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

#[wasm_bindgen(js_name=validateAssetLockTransaction)]
pub async fn validate_asset_lock_transaction(
    state_repository: ExternalStateRepositoryLike,
    raw_transaction: String,
    output_index: usize,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let tx_bytes = Vec::from_hex(&raw_transaction)
        .map_err(|_| RustConversionError::Error(String::from("invalid transaction hex")))?;

    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let validator = AssetLockTransactionValidator::new(state_repository_wrapper);

    let result = validator
        .validate(tx_bytes.as_slice(), output_index, execution_context.into())
        .await
        .map_err(|e| from_dpp_err(e.into()))?;

    let validation_result = result.map(|item| {
        let object = js_sys::Object::new();

        js_sys::Reflect::set(
            &object,
            &"publicKeyHash".to_owned().into(),
            &Buffer::from_bytes(&item.public_key_hash),
        )
        .unwrap();

        let deserialized_tx = consensus::serialize(&item.transaction);

        js_sys::Reflect::set(
            &object,
            &"transaction".to_owned().into(),
            &Buffer::from_bytes(&deserialized_tx),
        )
        .unwrap();

        object
    });

    Ok(validation_result.into())
}
