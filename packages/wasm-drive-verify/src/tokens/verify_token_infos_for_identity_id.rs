use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::identifier_to_base58;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyTokenInfosForIdentityIdResult {
    root_hash: Vec<u8>,
    token_infos: JsValue,
}

#[wasm_bindgen]
impl VerifyTokenInfosForIdentityIdResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn token_infos(&self) -> JsValue {
        self.token_infos.clone()
    }
}

fn convert_token_info_to_js(info: &IdentityTokenInfo) -> Result<JsValue, JsValue> {
    // IdentityTokenInfo only has a frozen field
    let info_value = match info {
        IdentityTokenInfo::V0(v0) => {
            serde_json::json!({"frozen": v0.frozen})
        }
    };
    to_value(&info_value).map_err(|e| {
        JsValue::from_str(&format!("Failed to convert token info to JsValue: {:?}", e))
    })
}

// Vec variant - returns array of tuples [tokenId, tokenInfo]
#[wasm_bindgen(js_name = "verifyTokenInfosForIdentityIdVec")]
pub fn verify_token_infos_for_identity_id_vec(
    proof: &Uint8Array,
    token_ids: &JsValue,
    identity_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenInfosForIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

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

    let (root_hash, token_infos_vec): (RootHash, Vec<([u8; 32], Option<IdentityTokenInfo>)>) =
        drive::drive::Drive::verify_token_infos_for_identity_id(
            &proof_vec,
            &token_ids_vec,
            identity_id_bytes,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (id, info_option) in token_infos_vec {
        let tuple_array = Array::new();

        // Add token ID as Uint8Array
        let id_uint8 = Uint8Array::from(&id[..]);
        tuple_array.push(&id_uint8);

        // Add token info
        match info_option {
            Some(info) => {
                tuple_array.push(&convert_token_info_to_js(&info)?);
            }
            None => {
                tuple_array.push(&JsValue::NULL);
            }
        }

        js_array.push(&tuple_array);
    }

    Ok(VerifyTokenInfosForIdentityIdResult {
        root_hash: root_hash.to_vec(),
        token_infos: js_array.into(),
    })
}

// BTreeMap variant - returns object with token ID (base58) as key
#[wasm_bindgen(js_name = "verifyTokenInfosForIdentityIdMap")]
pub fn verify_token_infos_for_identity_id_map(
    proof: &Uint8Array,
    token_ids: &JsValue,
    identity_id: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyTokenInfosForIdentityIdResult, JsValue> {
    let proof_vec = proof.to_vec();

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

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

    let (root_hash, token_infos_map): (RootHash, BTreeMap<[u8; 32], Option<IdentityTokenInfo>>) =
        drive::drive::Drive::verify_token_infos_for_identity_id(
            &proof_vec,
            &token_ids_vec,
            identity_id_bytes,
            verify_subset_of_proof,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (id, info_option) in token_infos_map {
        let base58_key = identifier_to_base58(&id);

        let info_js = match info_option {
            Some(info) => convert_token_info_to_js(&info)?,
            None => JsValue::NULL,
        };

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &info_js)
            .map_err(|_| JsValue::from_str("Failed to set token info in result object"))?;
    }

    Ok(VerifyTokenInfosForIdentityIdResult {
        root_hash: root_hash.to_vec(),
        token_infos: js_obj.into(),
    })
}
