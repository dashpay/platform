mod chain;
mod instant;

pub use chain::*;
pub use instant::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen::__rt::Ref;

use crate::errors::{from_dpp_err, RustConversionError};
use dpp::dashcore::consensus;
use dpp::dashcore::hashes::hex::FromHex;
use dpp::identity::errors::UnknownAssetLockProofTypeError;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::errors::dpp_error::from_dpp_error_ref;
use crate::identifier::IdentifierWrapper;
use crate::identity::errors::UnknownAssetLockProofTypeErrorWasm;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::utils::generic_of_js_val;
use crate::validation::ValidationResultWasm;
use crate::{Deserialize, StateTransitionExecutionContextWasm};
use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockProof, AssetLockPublicKeyHashFetcher, AssetLockTransactionOutputFetcher,
    AssetLockTransactionValidator,
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

pub fn create_asset_lock_proof_from_wasm_instance(
    js_value: &JsValue,
) -> Result<AssetLockProof, JsValue> {
    let default_error: UnknownAssetLockProofTypeErrorWasm =
        UnknownAssetLockProofTypeError::new(None).into();

    let get_type_value = js_sys::Reflect::get(js_value, &JsValue::from_str("getType"))
        .map_err(|_| default_error.clone())?;

    let get_type_function: &js_sys::Function = get_type_value
        .dyn_ref::<js_sys::Function>()
        .ok_or(default_error.clone())?;

    let lock_type = get_type_function
        .call0(&js_value)?
        .as_f64()
        .ok_or(default_error.clone())? as u8;

    if lock_type == 0 {
        let instant: Ref<InstantAssetLockProofWasm> =
            generic_of_js_val::<InstantAssetLockProofWasm>(js_value, "InstantAssetLockProof")?;

        Ok(AssetLockProof::Instant(instant.clone().into()))
    } else if lock_type == 1 {
        let chain: Ref<ChainAssetLockProofWasm> =
            generic_of_js_val::<ChainAssetLockProofWasm>(js_value, "ChainAssetLockProof")?;

        Ok(AssetLockProof::Chain(chain.clone().into()))
    } else {
        Err(
            UnknownAssetLockProofTypeErrorWasm::from(UnknownAssetLockProofTypeError::new(Some(
                lock_type,
            )))
            .into(),
        )
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

#[wasm_bindgen(js_name=fetchAssetLockTransactionOutput)]
pub async fn fetch_asset_lock_transaction_output(
    state_repository: ExternalStateRepositoryLike,
    raw_asset_lock_proof: JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<JsValue, JsValue> {
    let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&raw_asset_lock_proof)?;

    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let fetcher = AssetLockTransactionOutputFetcher::new(state_repository_wrapper);

    let fetch_result = fetcher
        .fetch(&asset_lock_proof, execution_context.into())
        .await
        .map_err(|e| from_dpp_error_ref(&e))?;

    let tx_out = js_sys::Object::new();

    js_sys::Reflect::set(
        &tx_out,
        &"satoshis".to_owned().into(),
        &(fetch_result.value as u32).into(),
    )
    .unwrap();

    let script_hex = format!("{:x}", fetch_result.script_pubkey);

    js_sys::Reflect::set(&tx_out, &"script".to_owned().into(), &script_hex.into()).unwrap();

    Ok(tx_out.into())
}

#[wasm_bindgen(js_name=fetchAssetLockPublicKeyHash)]
pub async fn fetch_asset_lock_public_key_hash(
    state_repository: ExternalStateRepositoryLike,
    raw_asset_lock_proof: JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<JsValue, JsValue> {
    let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&raw_asset_lock_proof)?;

    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let tx_output_fetcher =
        AssetLockTransactionOutputFetcher::new(state_repository_wrapper.clone());

    let public_key_hash_fetcher =
        AssetLockPublicKeyHashFetcher::new(state_repository_wrapper, tx_output_fetcher);

    let fetch_result = public_key_hash_fetcher
        .fetch_public_key_hash(asset_lock_proof, execution_context.into())
        .await
        .map_err(|e| from_dpp_error_ref(&e))?;

    Ok(Buffer::from_bytes(&fetch_result).into())
}
