use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::bytes_to_base58;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::proposer_block_count_query::ProposerQueryType;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyEpochProposersResult {
    root_hash: Vec<u8>,
    proposers: JsValue,
}

#[wasm_bindgen]
impl VerifyEpochProposersResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn proposers(&self) -> JsValue {
        self.proposers.clone()
    }
}

// Vec variant - returns array of tuples [proposerId, blockCount]
#[wasm_bindgen(js_name = "verifyEpochProposersByRangeVec")]
pub fn verify_epoch_proposers_by_range_vec(
    proof: &Uint8Array,
    epoch_index: u16,
    limit: Option<u16>,
    start_at_proposer_id: Option<Uint8Array>,
    start_at_included: Option<bool>,
    platform_version_number: u32,
) -> Result<VerifyEpochProposersResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse start_at
    let start_at = match (start_at_proposer_id, start_at_included) {
        (Some(id), included) => {
            let id_vec = id.to_vec();
            let id_bytes: [u8; 32] = id_vec
                .try_into()
                .map_err(|_| JsValue::from_str("Invalid proposer ID length. Expected 32 bytes."))?;
            Some((id_bytes, included.unwrap_or(true)))
        }
        _ => None,
    };

    let proposer_query_type = ProposerQueryType::ByRange(limit, start_at);

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, proposers_vec): (RootHash, Vec<(Vec<u8>, u64)>) =
        Drive::verify_epoch_proposers(
            &proof_vec,
            epoch_index,
            proposer_query_type,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (proposer_id, block_count) in proposers_vec {
        let tuple_array = Array::new();

        // Add proposer ID as Uint8Array
        let id_uint8 = Uint8Array::from(&proposer_id[..]);
        tuple_array.push(&id_uint8);

        // Add block count
        tuple_array.push(&JsValue::from_f64(block_count as f64));

        js_array.push(&tuple_array);
    }

    Ok(VerifyEpochProposersResult {
        root_hash: root_hash.to_vec(),
        proposers: js_array.into(),
    })
}

// BTreeMap variant - returns object with proposer ID (base58) as key
#[wasm_bindgen(js_name = "verifyEpochProposersByRangeMap")]
pub fn verify_epoch_proposers_by_range_map(
    proof: &Uint8Array,
    epoch_index: u16,
    limit: Option<u16>,
    start_at_proposer_id: Option<Uint8Array>,
    start_at_included: Option<bool>,
    platform_version_number: u32,
) -> Result<VerifyEpochProposersResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse start_at
    let start_at = match (start_at_proposer_id, start_at_included) {
        (Some(id), included) => {
            let id_vec = id.to_vec();
            let id_bytes: [u8; 32] = id_vec
                .try_into()
                .map_err(|_| JsValue::from_str("Invalid proposer ID length. Expected 32 bytes."))?;
            Some((id_bytes, included.unwrap_or(true)))
        }
        _ => None,
    };

    let proposer_query_type = ProposerQueryType::ByRange(limit, start_at);

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, proposers_map): (RootHash, BTreeMap<Vec<u8>, u64>) =
        Drive::verify_epoch_proposers(
            &proof_vec,
            epoch_index,
            proposer_query_type,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (proposer_id, block_count) in proposers_map {
        let base58_key = bytes_to_base58(&proposer_id);

        Reflect::set(
            &js_obj,
            &JsValue::from_str(&base58_key),
            &JsValue::from_f64(block_count as f64),
        )
        .map_err(|_| JsValue::from_str("Failed to set proposer in result object"))?;
    }

    Ok(VerifyEpochProposersResult {
        root_hash: root_hash.to_vec(),
        proposers: js_obj.into(),
    })
}

// Vec variant for ByIds query - returns array of tuples [proposerId, blockCount]
#[wasm_bindgen(js_name = "verifyEpochProposersByIdsVec")]
pub fn verify_epoch_proposers_by_ids_vec(
    proof: &Uint8Array,
    epoch_index: u16,
    proposer_ids: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyEpochProposersResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse proposer IDs from JS array
    let ids_array: Array = proposer_ids
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("proposer_ids must be an array"))?;

    let mut proposer_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each proposer ID must be a Uint8Array"))?;

        proposer_ids_vec.push(id_uint8.to_vec());
    }

    let proposer_query_type = ProposerQueryType::ByIds(proposer_ids_vec);

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, proposers_vec): (RootHash, Vec<(Vec<u8>, u64)>) =
        Drive::verify_epoch_proposers(
            &proof_vec,
            epoch_index,
            proposer_query_type,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (proposer_id, block_count) in proposers_vec {
        let tuple_array = Array::new();

        // Add proposer ID as Uint8Array
        let id_uint8 = Uint8Array::from(&proposer_id[..]);
        tuple_array.push(&id_uint8);

        // Add block count
        tuple_array.push(&JsValue::from_f64(block_count as f64));

        js_array.push(&tuple_array);
    }

    Ok(VerifyEpochProposersResult {
        root_hash: root_hash.to_vec(),
        proposers: js_array.into(),
    })
}

// BTreeMap variant for ByIds query - returns object with proposer ID (base58) as key
#[wasm_bindgen(js_name = "verifyEpochProposersByIdsMap")]
pub fn verify_epoch_proposers_by_ids_map(
    proof: &Uint8Array,
    epoch_index: u16,
    proposer_ids: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyEpochProposersResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse proposer IDs from JS array
    let ids_array: Array = proposer_ids
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("proposer_ids must be an array"))?;

    let mut proposer_ids_vec = Vec::new();
    for i in 0..ids_array.length() {
        let id_array = ids_array.get(i);
        let id_uint8: Uint8Array = id_array
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each proposer ID must be a Uint8Array"))?;

        proposer_ids_vec.push(id_uint8.to_vec());
    }

    let proposer_query_type = ProposerQueryType::ByIds(proposer_ids_vec);

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, proposers_map): (RootHash, BTreeMap<Vec<u8>, u64>) =
        Drive::verify_epoch_proposers(
            &proof_vec,
            epoch_index,
            proposer_query_type,
            platform_version,
        )
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (proposer_id, block_count) in proposers_map {
        let base58_key = bytes_to_base58(&proposer_id);

        Reflect::set(
            &js_obj,
            &JsValue::from_str(&base58_key),
            &JsValue::from_f64(block_count as f64),
        )
        .map_err(|_| JsValue::from_str("Failed to set proposer in result object"))?;
    }

    Ok(VerifyEpochProposersResult {
        root_hash: root_hash.to_vec(),
        proposers: js_obj.into(),
    })
}
