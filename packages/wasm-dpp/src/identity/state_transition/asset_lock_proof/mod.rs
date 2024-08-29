mod chain;
mod instant;

pub use chain::*;
pub use instant::*;
use std::convert::TryInto;
use wasm_bindgen::JsCast;
use wasm_bindgen::__rt::Ref;

use dpp::identity::errors::UnknownAssetLockProofTypeError;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;
use crate::identity::errors::UnknownAssetLockProofTypeErrorWasm;

use crate::utils::generic_of_js_val;

use dpp::identity::state_transition::asset_lock_proof::{AssetLockProof, AssetLockProofType};
use serde::Deserialize;

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
        let lock_type = get_type_from_raw_asset_lock_proof(&raw_asset_lock_proof)?;

        match lock_type {
            AssetLockProofType::Instant => {
                Ok(InstantAssetLockProofWasm::new(raw_asset_lock_proof)?.into())
            }
            AssetLockProofType::Chain => {
                Ok(ChainAssetLockProofWasm::new(raw_asset_lock_proof)?.into())
            }
        }
    }

    #[wasm_bindgen(js_name=createIdentifier)]
    pub fn create_identifier(&self) -> Result<IdentifierWrapper, JsValue> {
        match &self.0 {
            AssetLockProof::Instant(instant) => {
                InstantAssetLockProofWasm::from(instant.to_owned()).create_identifier()
            }
            AssetLockProof::Chain(chain) => {
                let identifier =
                    ChainAssetLockProofWasm::from(chain.to_owned()).create_identifier();
                Ok(identifier)
            }
        }
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        match &self.0 {
            AssetLockProof::Instant(instant) => {
                InstantAssetLockProofWasm::from(instant.to_owned()).to_object()
            }
            AssetLockProof::Chain(chain) => {
                ChainAssetLockProofWasm::from(chain.to_owned()).to_object()
            }
        }
    }
}

fn get_type_from_raw_asset_lock_proof(
    raw_asset_lock_proof: &JsValue,
) -> Result<AssetLockProofType, JsError> {
    let proof_type = js_sys::Reflect::get(raw_asset_lock_proof, &JsValue::from_str("type"))
        .map_err(|_| JsError::new("error getting type from raw asset lock"))?
        .as_f64()
        .ok_or_else(|| JsError::new("asset lock type must be a number"))?;

    (proof_type as u64)
        .try_into()
        .map_err(|_| JsError::new("unrecognized asset lock proof type"))
}

#[wasm_bindgen(js_name=createAssetLockProofInstance)]
pub fn create_asset_lock_proof_instance(raw_parameters: JsValue) -> Result<JsValue, JsError> {
    let lock_type = get_type_from_raw_asset_lock_proof(&raw_parameters)?;

    match lock_type {
        AssetLockProofType::Instant => {
            InstantAssetLockProofWasm::new(raw_parameters).map(|v| v.into())
        }
        AssetLockProofType::Chain => ChainAssetLockProofWasm::new(raw_parameters).map(|v| v.into()),
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
        .ok_or_else(|| default_error.clone())?;

    let raw_lock_type = get_type_function
        .call0(js_value)?
        .as_f64()
        .ok_or(default_error)? as u64;

    let lock_type: AssetLockProofType = raw_lock_type.try_into().map_err(|_| {
        UnknownAssetLockProofTypeErrorWasm::from(UnknownAssetLockProofTypeError::new(Some(
            raw_lock_type as u8,
        )))
    })?;

    match lock_type {
        AssetLockProofType::Instant => {
            let instant: Ref<InstantAssetLockProofWasm> =
                generic_of_js_val::<InstantAssetLockProofWasm>(js_value, "InstantAssetLockProof")?;

            Ok(AssetLockProof::Instant(instant.clone().into()))
        }
        AssetLockProofType::Chain => {
            let chain: Ref<ChainAssetLockProofWasm> =
                generic_of_js_val::<ChainAssetLockProofWasm>(js_value, "ChainAssetLockProof")?;

            Ok(AssetLockProof::Chain(chain.clone().into()))
        }
    }
}

// TODO(versioning): restore?
// #[wasm_bindgen(js_name=validateAssetLockTransaction)]
// pub async fn validate_asset_lock_transaction(
//     state_repository: ExternalStateRepositoryLike,
//     raw_transaction: String,
//     output_index: usize,
//     execution_context: &StateTransitionExecutionContextWasm,
// ) -> Result<ValidationResultWasm, JsValue> {
//     let tx_bytes = Vec::from_hex(&raw_transaction)
//         .map_err(|_| RustConversionError::Error(String::from("invalid transaction hex")))?;
//
//     let state_repository_wrapper =
//         Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));
//
//     let validator = AssetLockTransactionValidator::new(state_repository_wrapper);
//
//     let result = validator
//         .validate(tx_bytes.as_slice(), output_index, execution_context.into())
//         .await
//         .map_err(|e| from_dpp_err(e.into()))?;
//
//     let validation_result = result.map(|item| {
//         let object = js_sys::Object::new();
//
//         js_sys::Reflect::set(
//             &object,
//             &"publicKeyHash".to_owned().into(),
//             &Buffer::from_bytes(&item.public_key_hash),
//         )
//         .unwrap();
//
//         let deserialized_tx = consensus::serialize(&item.transaction);
//
//         js_sys::Reflect::set(
//             &object,
//             &"transaction".to_owned().into(),
//             &Buffer::from_bytes(&deserialized_tx),
//         )
//         .unwrap();
//
//         object
//     });
//
//     Ok(validation_result.into())
// }

// TODO(versioning): restore?
// #[wasm_bindgen(js_name=fetchAssetLockTransactionOutput)]
// pub async fn fetch_asset_lock_transaction_output(
//     state_repository: ExternalStateRepositoryLike,
//     raw_asset_lock_proof: JsValue,
//     execution_context: &StateTransitionExecutionContextWasm,
// ) -> Result<JsValue, JsValue> {
//     let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&raw_asset_lock_proof)?;
//
//     let state_repository_wrapper =
//         Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));
//
//     let fetcher = AssetLockTransactionOutputFetcher::new(state_repository_wrapper);
//
//     let fetch_result = fetcher
//         .fetch(&asset_lock_proof, execution_context.into())
//         .await
//         .map_err(|e| from_dpp_error_ref(&e))?;
//
//     let tx_out = js_sys::Object::new();
//
//     js_sys::Reflect::set(
//         &tx_out,
//         &"satoshis".to_owned().into(),
//         &(fetch_result.value as u32).into(),
//     )
//     .unwrap();
//
//     let script_hex = format!("{:x}", fetch_result.script_pubkey);
//
//     js_sys::Reflect::set(&tx_out, &"script".to_owned().into(), &script_hex.into()).unwrap();
//
//     Ok(tx_out.into())
// }

// TODO(versioning): restore?
// #[wasm_bindgen(js_name=fetchAssetLockPublicKeyHash)]
// pub async fn fetch_asset_lock_public_key_hash(
//     state_repository: ExternalStateRepositoryLike,
//     raw_asset_lock_proof: JsValue,
//     execution_context: &StateTransitionExecutionContextWasm,
// ) -> Result<JsValue, JsValue> {
//     let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&raw_asset_lock_proof)?;
//
//     let state_repository_wrapper =
//         Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));
//
//     let tx_output_fetcher =
//         AssetLockTransactionOutputFetcher::new(state_repository_wrapper.clone());
//
//     let public_key_hash_fetcher =
//         AssetLockPublicKeyHashFetcher::new(state_repository_wrapper, Arc::new(tx_output_fetcher));
//
//     let fetch_result = public_key_hash_fetcher
//         .fetch_public_key_hash(asset_lock_proof, execution_context.into())
//         .await
//         .map_err(|e| from_dpp_error_ref(&e))?;
//
//     Ok(Buffer::from_bytes(&fetch_result).into())
// }
