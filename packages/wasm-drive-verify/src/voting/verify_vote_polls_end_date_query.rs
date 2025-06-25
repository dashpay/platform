use crate::utils::getters::VecU8ToUint8Array;
use bincode;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use drive::query::VotePollsByEndDateDriveQuery;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyVotePollsEndDateQueryResult {
    root_hash: Vec<u8>,
    vote_polls: JsValue,
}

#[wasm_bindgen]
impl VerifyVotePollsEndDateQueryResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn vote_polls(&self) -> JsValue {
        self.vote_polls.clone()
    }
}

// Vec variant - returns array of tuples [timestamp, votePolls[]]
#[wasm_bindgen(js_name = "verifyVotePollsEndDateQueryVec")]
pub fn verify_vote_polls_end_date_query_vec(
    proof: &Uint8Array,
    query_cbor: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyVotePollsEndDateQueryResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Deserialize the query using bincode
    let query: VotePollsByEndDateDriveQuery =
        bincode::decode_from_slice(&query_cbor.to_vec(), bincode::config::standard())
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize query: {:?}", e)))?
            .0;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, polls_vec): (RootHash, Vec<(TimestampMillis, Vec<VotePoll>)>) = query
        .verify_vote_polls_by_end_date_proof(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (timestamp, vote_polls) in polls_vec {
        let tuple_array = Array::new();

        // Add timestamp as number
        tuple_array.push(&JsValue::from_f64(timestamp as f64));

        // Add vote polls as array of CBOR-encoded polls
        let polls_array = Array::new();
        for poll in vote_polls {
            let mut poll_bytes = Vec::new();
            ciborium::into_writer(&poll, &mut poll_bytes).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize vote poll: {:?}", e))
            })?;
            let poll_uint8 = Uint8Array::from(&poll_bytes[..]);
            polls_array.push(&poll_uint8);
        }
        tuple_array.push(&polls_array);

        js_array.push(&tuple_array);
    }

    Ok(VerifyVotePollsEndDateQueryResult {
        root_hash: root_hash.to_vec(),
        vote_polls: js_array.into(),
    })
}

// BTreeMap variant - returns object with timestamp as key
#[wasm_bindgen(js_name = "verifyVotePollsEndDateQueryMap")]
pub fn verify_vote_polls_end_date_query_map(
    proof: &Uint8Array,
    query_cbor: &Uint8Array,
    platform_version_number: u32,
) -> Result<VerifyVotePollsEndDateQueryResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Deserialize the query using bincode
    let query: VotePollsByEndDateDriveQuery =
        bincode::decode_from_slice(&query_cbor.to_vec(), bincode::config::standard())
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize query: {:?}", e)))?
            .0;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, polls_map): (RootHash, BTreeMap<TimestampMillis, Vec<VotePoll>>) = query
        .verify_vote_polls_by_end_date_proof(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with timestamp as string key
    let js_obj = Object::new();
    for (timestamp, vote_polls) in polls_map {
        let timestamp_key = timestamp.to_string();

        // Convert vote polls to array
        let polls_array = Array::new();
        for poll in vote_polls {
            let mut poll_bytes = Vec::new();
            ciborium::into_writer(&poll, &mut poll_bytes).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize vote poll: {:?}", e))
            })?;
            let poll_uint8 = Uint8Array::from(&poll_bytes[..]);
            polls_array.push(&poll_uint8);
        }

        Reflect::set(&js_obj, &JsValue::from_str(&timestamp_key), &polls_array)
            .map_err(|_| JsValue::from_str("Failed to set vote polls in result object"))?;
    }

    Ok(VerifyVotePollsEndDateQueryResult {
        root_hash: root_hash.to_vec(),
        vote_polls: js_obj.into(),
    })
}
