use crate::asset_lock_proof::instant::instant_lock::InstantLockWasm;
use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::asset_lock_proof::tx_out::TxOutWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::{JsValueExt, js_value_to_vec_u8};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use dpp::dashcore::consensus::{deserialize, serialize};
use dpp::dashcore::{InstantLock, Transaction};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use js_sys::{Object, Reflect};
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "InstantAssetLockProof")]
pub struct InstantAssetLockProofWasm(InstantAssetLockProof);

impl From<InstantAssetLockProofWasm> for InstantAssetLockProof {
    fn from(proof: InstantAssetLockProofWasm) -> Self {
        proof.0
    }
}

impl From<InstantAssetLockProof> for InstantAssetLockProofWasm {
    fn from(proof: InstantAssetLockProof) -> Self {
        InstantAssetLockProofWasm(proof)
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProof)]
impl InstantAssetLockProofWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "InstantAssetLockProof".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "InstantAssetLockProof".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        instant_lock: Vec<u8>,
        transaction: Vec<u8>,
        output_index: u32,
    ) -> WasmDppResult<InstantAssetLockProofWasm> {
        let instant_lock: InstantLock = deserialize(instant_lock.as_slice())
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        let transaction: Transaction = deserialize(transaction.as_slice())
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        Ok(InstantAssetLockProofWasm(InstantAssetLockProof {
            instant_lock,
            transaction,
            output_index,
        }))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: JsValue) -> WasmDppResult<InstantAssetLockProofWasm> {
        let (instant_lock, transaction, output_index) =
            parse_instant_asset_lock_proof_fields(value)?;
        InstantAssetLockProofWasm::new(instant_lock, transaction, output_index)
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        // Use default serializer (non-human-readable) - bytes stay as Uint8Array
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "getOutput")]
    pub fn get_output(&self) -> Option<TxOutWasm> {
        self.0.output().map(|output| output.clone().into())
    }

    #[wasm_bindgen(js_name = "getOutPoint")]
    pub fn get_out_point(&self) -> Option<OutPointWasm> {
        self.0.out_point().map(|output| output.into())
    }

    #[wasm_bindgen(getter = "outputIndex")]
    pub fn get_output_index(&self) -> u32 {
        self.0.output_index()
    }

    #[wasm_bindgen(getter = "instantLock")]
    pub fn get_instant_lock(&self) -> InstantLockWasm {
        self.0.instant_lock.clone().into()
    }

    #[wasm_bindgen(setter = "outputIndex")]
    pub fn set_output_index(&mut self, output_index: u32) {
        self.0.output_index = output_index;
    }

    #[wasm_bindgen(setter = "instantLock")]
    pub fn set_instant_lock(&mut self, instant_lock: &InstantLockWasm) {
        self.0.instant_lock = instant_lock.clone().into();
    }

    #[wasm_bindgen(js_name=getTransaction)]
    pub fn get_transaction(&self) -> Vec<u8> {
        let transaction = self.0.transaction();
        serialize(transaction)
    }

    #[wasm_bindgen(js_name=getInstantLockBytes)]
    pub fn get_instant_lock_bytes(&self) -> Vec<u8> {
        let instant_lock = self.0.instant_lock();
        serialize(instant_lock)
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> WasmDppResult<IdentifierWasm> {
        let identifier = self.0.create_identifier()?;

        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        // Serialize to serde_json::Value first (human-readable, so BinaryData becomes base64)
        // then convert to JS value
        let json_value = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(value: JsValue) -> WasmDppResult<InstantAssetLockProofWasm> {
        let object = value.dyn_into::<Object>().map_err(|_| {
            WasmDppError::invalid_argument("InstantAssetLockProof expects an object".to_string())
        })?;

        let instant_lock_js =
            Reflect::get(&object, &JsValue::from_str("instantLock")).map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "unable to read instantLock: {}",
                    err.error_message()
                ))
            })?;
        let transaction_js =
            Reflect::get(&object, &JsValue::from_str("transaction")).map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "unable to read transaction: {}",
                    err.error_message()
                ))
            })?;
        let output_index_js =
            Reflect::get(&object, &JsValue::from_str("outputIndex")).map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "unable to read outputIndex: {}",
                    err.error_message()
                ))
            })?;

        let output_index = output_index_js
            .as_f64()
            .ok_or_else(|| {
                WasmDppError::invalid_argument("outputIndex must be a number".to_string())
            })? as u32;

        // Parse base64 strings
        let instant_lock_base64 = instant_lock_js.as_string().ok_or_else(|| {
            WasmDppError::invalid_argument("instantLock must be a base64 string".to_string())
        })?;
        let transaction_base64 = transaction_js.as_string().ok_or_else(|| {
            WasmDppError::invalid_argument("transaction must be a base64 string".to_string())
        })?;

        let instant_lock = BASE64_STANDARD.decode(&instant_lock_base64).map_err(|e| {
            WasmDppError::invalid_argument(format!("invalid base64 instantLock: {}", e))
        })?;
        let transaction = BASE64_STANDARD.decode(&transaction_base64).map_err(|e| {
            WasmDppError::invalid_argument(format!("invalid base64 transaction: {}", e))
        })?;

        InstantAssetLockProofWasm::new(instant_lock, transaction, output_index)
    }
}

fn parse_instant_asset_lock_proof_fields(value: JsValue) -> WasmDppResult<(Vec<u8>, Vec<u8>, u32)> {
    let object = value.dyn_into::<Object>().map_err(|_| {
        WasmDppError::invalid_argument("InstantAssetLockProof expects an object".to_string())
    })?;

    let instant_lock_js =
        Reflect::get(&object, &JsValue::from_str("instantLock")).map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "unable to read instantLock: {}",
                err.error_message()
            ))
        })?;
    let transaction_js =
        Reflect::get(&object, &JsValue::from_str("transaction")).map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "unable to read transaction: {}",
                err.error_message()
            ))
        })?;
    let output_index_js =
        Reflect::get(&object, &JsValue::from_str("outputIndex")).map_err(|err| {
            WasmDppError::invalid_argument(format!(
                "unable to read outputIndex: {}",
                err.error_message()
            ))
        })?;

    let output_index = output_index_js
        .as_f64()
        .ok_or_else(|| WasmDppError::invalid_argument("outputIndex must be a number".to_string()))?
        as u32;

    let instant_lock = js_value_to_vec_u8(&instant_lock_js)?;
    let transaction = js_value_to_vec_u8(&transaction_js)?;

    Ok((instant_lock, transaction, output_index))
}
