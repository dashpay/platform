use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::identifier_to_base58;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyIdentityBalancesForIdentityIdsResult {
    root_hash: Vec<u8>,
    balances: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentityBalancesForIdentityIdsResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn balances(&self) -> JsValue {
        self.balances.clone()
    }
}

// Vec variant - returns array of tuples [identityId, balance]
#[wasm_bindgen(js_name = "verifyIdentityBalancesForIdentityIdsVec")]
pub fn verify_identity_balances_for_identity_ids_vec(
    proof: &Uint8Array,
    is_proof_subset: bool,
    identity_ids: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityBalancesForIdentityIdsResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse identity IDs from JS array
    let ids_array: Array = identity_ids
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("identity_ids must be an array"))?;

    let mut identity_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each identity ID must be a Uint8Array"))?;

        let id_vec = id_uint8.to_vec();
        let id_bytes: [u8; 32] = id_vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid identity ID length. Expected 32 bytes."))?;

        identity_ids_vec.push(id_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, balances_vec): (RootHash, Vec<([u8; 32], Option<Credits>)>) =
        Drive::verify_identity_balances_for_identity_ids(
            &proof_vec,
            is_proof_subset,
            &identity_ids_vec,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (id, balance_option) in balances_vec {
        let tuple_array = Array::new();

        // Add identity ID as Uint8Array
        let id_uint8 = Uint8Array::from(&id[..]);
        tuple_array.push(&id_uint8);

        // Add balance
        match balance_option {
            Some(credits) => {
                tuple_array.push(&JsValue::from_str(&credits.to_string()));
            }
            None => {
                tuple_array.push(&JsValue::NULL);
            }
        }

        js_array.push(&tuple_array);
    }

    Ok(VerifyIdentityBalancesForIdentityIdsResult {
        root_hash: root_hash.to_vec(),
        balances: js_array.into(),
    })
}

// BTreeMap variant - returns object with identity ID (base58) as key
#[wasm_bindgen(js_name = "verifyIdentityBalancesForIdentityIdsMap")]
pub fn verify_identity_balances_for_identity_ids_map(
    proof: &Uint8Array,
    is_proof_subset: bool,
    identity_ids: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityBalancesForIdentityIdsResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse identity IDs from JS array
    let ids_array: Array = identity_ids
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("identity_ids must be an array"))?;

    let mut identity_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each identity ID must be a Uint8Array"))?;

        let id_vec = id_uint8.to_vec();
        let id_bytes: [u8; 32] = id_vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid identity ID length. Expected 32 bytes."))?;

        identity_ids_vec.push(id_bytes);
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, balances_map): (RootHash, BTreeMap<[u8; 32], Option<Credits>>) =
        Drive::verify_identity_balances_for_identity_ids(
            &proof_vec,
            is_proof_subset,
            &identity_ids_vec,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (id, balance_option) in balances_map {
        let base58_key = identifier_to_base58(&id);

        let balance_js = match balance_option {
            Some(credits) => JsValue::from_str(&credits.to_string()),
            None => JsValue::NULL,
        };

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &balance_js)
            .map_err(|_| JsValue::from_str("Failed to set balance in result object"))?;
    }

    Ok(VerifyIdentityBalancesForIdentityIdsResult {
        root_hash: root_hash.to_vec(),
        balances: js_obj.into(),
    })
}
