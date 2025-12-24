use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::identifier_to_base58;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenStatusesResult {
    root_hash: Vec<u8>,
    statuses: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenStatusesResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn statuses(&self) -> JsValue {
        self.statuses.clone()
    }
}

// Vec variant - returns array of tuples [tokenId, status]
#[wasm_bindgen(js_name = "verifyTokenStatusesVec")]
pub fn verify_token_statuses_vec(
    proof: &Uint8Array,
    token_ids: &JsValue,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenStatusesResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse token IDs from JS array
    let ids_array: Array = token_ids
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("token_ids must be an array"))?;

    let mut token_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each token ID must be a Uint8Array"))?;

        let id_vec = id_uint8.to_vec();
        let id_bytes: [u8; 32] = id_vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid token ID length. Expected 32 bytes."))?;

        token_ids_vec.push(id_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, statuses_vec): (RootHash, Vec<([u8; 32], Option<TokenStatus>)>) =
        drive::drive::Drive::verify_token_statuses(
            &proof_vec,
            &token_ids_vec,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (id, status_option) in statuses_vec {
        let tuple_array = Array::new();

        // Add token ID as Uint8Array
        let id_uint8 = Uint8Array::from(&id[..]);
        tuple_array.push(&id_uint8);

        // Add status
        match status_option {
            Some(status) => {
                let status_value = match status {
                    TokenStatus::V0(v0) => {
                        serde_json::json!({"paused": v0.paused})
                    }
                };
                let status_js = to_value(&status_value).unwrap_or(JsValue::NULL);
                tuple_array.push(&status_js);
            }
            None => {
                tuple_array.push(&JsValue::NULL);
            }
        }

        js_array.push(&tuple_array);
    }

    Ok(VerifyTokenStatusesResult {
        root_hash: root_hash.to_vec(),
        statuses: js_array.into(),
    })
}

// BTreeMap variant - returns object with token ID (base58) as key
#[wasm_bindgen(js_name = "verifyTokenStatusesMap")]
pub fn verify_token_statuses_map(
    proof: &Uint8Array,
    token_ids: &JsValue,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenStatusesResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse token IDs from JS array
    let ids_array: Array = token_ids
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("token_ids must be an array"))?;

    let mut token_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each token ID must be a Uint8Array"))?;

        let id_vec = id_uint8.to_vec();
        let id_bytes: [u8; 32] = id_vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid token ID length. Expected 32 bytes."))?;

        token_ids_vec.push(id_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, statuses_map): (RootHash, BTreeMap<[u8; 32], Option<TokenStatus>>) =
        drive::drive::Drive::verify_token_statuses(
            &proof_vec,
            &token_ids_vec,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (id, status_option) in statuses_map {
        let base58_key = identifier_to_base58(&id);

        let status_js = match status_option {
            Some(status) => {
                let status_value = match status {
                    TokenStatus::V0(v0) => {
                        serde_json::json!({"paused": v0.paused})
                    }
                };
                to_value(&status_value).unwrap_or(JsValue::NULL)
            }
            None => JsValue::NULL,
        };

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &status_js)
            .map_err(|_| JsValue::from_str("Failed to set status in result object"))?;
    }

    Ok(VerifyTokenStatusesResult {
        root_hash: root_hash.to_vec(),
        statuses: js_obj.into(),
    })
}
