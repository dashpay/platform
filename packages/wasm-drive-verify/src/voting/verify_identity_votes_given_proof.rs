use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::serialization::identifier_to_base58;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use dpp::voting::votes::resource_vote::ResourceVote;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::ContractLookupFn;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

fn deserialize_contested_resource_votes_query(
    query_cbor: &Uint8Array,
) -> Result<ContestedResourceVotesGivenByIdentityQuery, JsValue> {
    // Deserialize the query components from CBOR
    let query_value: serde_json::Value = ciborium::de::from_reader(&query_cbor.to_vec()[..])
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize query: {:?}", e)))?;

    // Extract fields from the deserialized value
    let query_obj = query_value
        .as_object()
        .ok_or_else(|| JsValue::from_str("Query must be an object"))?;

    let identity_id_bytes: Vec<u8> = query_obj
        .get("identity_id")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .map(|v| v.as_u64().map(|n| n as u8))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| JsValue::from_str("Invalid identity_id in query"))?;

    let identity_id = Identifier::from_bytes(&identity_id_bytes)
        .map_err(|e| JsValue::from_str(&format!("Invalid identity_id: {:?}", e)))?;

    let offset = query_obj
        .get("offset")
        .and_then(|v| v.as_u64())
        .map(|n| n as u16);

    let limit = query_obj
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as u16);

    let start_at = query_obj
        .get("start_at")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            if arr.len() == 2 {
                let bytes_arr = arr[0].as_array()?;
                let bytes: Vec<u8> = bytes_arr
                    .iter()
                    .map(|v| v.as_u64().map(|n| n as u8))
                    .collect::<Option<Vec<_>>>()?;
                let bytes_32: [u8; 32] = bytes.try_into().ok()?;
                let included = arr[1].as_bool()?;
                Some((bytes_32, included))
            } else {
                None
            }
        });

    let order_ascending = query_obj
        .get("order_ascending")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    Ok(ContestedResourceVotesGivenByIdentityQuery {
        identity_id,
        offset,
        limit,
        start_at,
        order_ascending,
    })
}

#[wasm_bindgen]
pub struct VerifyIdentityVotesGivenProofResult {
    root_hash: Vec<u8>,
    votes: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentityVotesGivenProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn votes(&self) -> JsValue {
        self.votes.clone()
    }
}

// Vec variant - returns array of tuples [identifier, resourceVote]
#[wasm_bindgen(js_name = "verifyIdentityVotesGivenProofVec")]
pub fn verify_identity_votes_given_proof_vec(
    proof: &Uint8Array,
    query_cbor: &Uint8Array,
    contract_lookup: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityVotesGivenProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // Deserialize the query
    let query = deserialize_contested_resource_votes_query(query_cbor)?;

    // Create contract lookup function
    let contract_lookup_fn = create_contract_lookup_fn(contract_lookup, platform_version)?;

    let (root_hash, votes_vec): (RootHash, Vec<(Identifier, ResourceVote)>) = query
        .verify_identity_votes_given_proof(&proof_vec, &*contract_lookup_fn, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (identifier, resource_vote) in votes_vec {
        let tuple_array = Array::new();

        // Add identifier as Uint8Array
        let id_bytes = identifier.as_bytes();
        let id_uint8 = Uint8Array::from(&id_bytes[..]);
        tuple_array.push(&id_uint8);

        // Serialize resource vote to CBOR
        let mut vote_bytes = Vec::new();
        ciborium::into_writer(&resource_vote, &mut vote_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize vote: {:?}", e)))?;
        let vote_uint8 = Uint8Array::from(&vote_bytes[..]);
        tuple_array.push(&vote_uint8);

        js_array.push(&tuple_array);
    }

    Ok(VerifyIdentityVotesGivenProofResult {
        root_hash: root_hash.to_vec(),
        votes: js_array.into(),
    })
}

// BTreeMap variant - returns object with identifier (base58) as key
#[wasm_bindgen(js_name = "verifyIdentityVotesGivenProofMap")]
pub fn verify_identity_votes_given_proof_map(
    proof: &Uint8Array,
    query_cbor: &Uint8Array,
    contract_lookup: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityVotesGivenProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // Deserialize the query
    let query = deserialize_contested_resource_votes_query(query_cbor)?;

    // Create contract lookup function
    let contract_lookup_fn = create_contract_lookup_fn(contract_lookup, platform_version)?;

    let (root_hash, votes_map): (RootHash, BTreeMap<Identifier, ResourceVote>) = query
        .verify_identity_votes_given_proof(&proof_vec, &*contract_lookup_fn, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with base58 keys
    let js_obj = Object::new();
    for (identifier, resource_vote) in votes_map {
        let base58_key = identifier_to_base58(&identifier.to_buffer());

        // Serialize resource vote to CBOR
        let mut vote_bytes = Vec::new();
        ciborium::into_writer(&resource_vote, &mut vote_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize vote: {:?}", e)))?;
        let vote_uint8 = Uint8Array::from(&vote_bytes[..]);

        Reflect::set(&js_obj, &JsValue::from_str(&base58_key), &vote_uint8)
            .map_err(|_| JsValue::from_str("Failed to set vote in result object"))?;
    }

    Ok(VerifyIdentityVotesGivenProofResult {
        root_hash: root_hash.to_vec(),
        votes: js_obj.into(),
    })
}

// Helper function to create contract lookup function from JS object
fn create_contract_lookup_fn<'a>(
    contract_lookup: &JsValue,
    platform_version: &PlatformVersion,
) -> Result<Box<ContractLookupFn<'a>>, JsValue> {
    if !contract_lookup.is_object() {
        return Err(JsValue::from_str("contract_lookup must be an object"));
    }

    let contracts_obj: Object = contract_lookup
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("contract_lookup must be an object"))?;

    // Get all keys from the object
    let keys = Object::keys(&contracts_obj);
    let mut contracts_map: BTreeMap<Identifier, Arc<DataContract>> = BTreeMap::new();

    for i in 0..keys.length() {
        let key = keys.get(i);
        let contract_bytes_js = Reflect::get(&contracts_obj, &key)
            .map_err(|_| JsValue::from_str("Failed to get contract from lookup object"))?;

        let contract_uint8: Uint8Array = contract_bytes_js
            .dyn_into()
            .map_err(|_| JsValue::from_str("Contract value must be a Uint8Array"))?;

        let contract_bytes = contract_uint8.to_vec();

        // Deserialize the contract
        let contract = DataContract::versioned_deserialize(&contract_bytes, true, platform_version)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;

        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        let identifier = contract.id();
        contracts_map.insert(identifier, Arc::new(contract));
    }

    let lookup_fn: Box<ContractLookupFn<'a>> =
        Box::new(move |id: &Identifier| Ok(contracts_map.get(id).cloned()));

    Ok(lookup_fn)
}
