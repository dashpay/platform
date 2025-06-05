use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::votes::resource_vote::ResourceVote;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::ContractLookupFn;
use drive::verify::RootHash;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::collections::BTreeMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyIdentityVotesGivenProofResult {
    root_hash: Vec<u8>,
    votes: JsValue,
}

#[wasm_bindgen]
impl VerifyIdentityVotesGivenProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
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

    // Deserialize the query
    let query: ContestedResourceVotesGivenByIdentityQuery =
        ciborium::de::from_reader(&query_cbor.to_vec()[..])
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize query: {:?}", e)))?;

    // Create contract lookup function
    let contract_lookup_fn: ContractLookupFn = create_contract_lookup_fn(contract_lookup)?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, votes_vec): (RootHash, Vec<(Identifier, ResourceVote)>) = query
        .verify_identity_votes_given_proof(&proof_vec, &contract_lookup_fn, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS array of tuples
    let js_array = Array::new();
    for (identifier, resource_vote) in votes_vec {
        let tuple_array = Array::new();

        // Add identifier as Uint8Array
        let id_uint8 = Uint8Array::from(identifier.as_bytes());
        tuple_array.push(&id_uint8);

        // Serialize resource vote to CBOR
        let vote_bytes = ciborium::ser::into_vec(&resource_vote)
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

// BTreeMap variant - returns object with identifier (hex) as key
#[wasm_bindgen(js_name = "verifyIdentityVotesGivenProofMap")]
pub fn verify_identity_votes_given_proof_map(
    proof: &Uint8Array,
    query_cbor: &Uint8Array,
    contract_lookup: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyIdentityVotesGivenProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Deserialize the query
    let query: ContestedResourceVotesGivenByIdentityQuery =
        ciborium::de::from_reader(&query_cbor.to_vec()[..])
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize query: {:?}", e)))?;

    // Create contract lookup function
    let contract_lookup_fn: ContractLookupFn = create_contract_lookup_fn(contract_lookup)?;

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, votes_map): (RootHash, BTreeMap<Identifier, ResourceVote>) = query
        .verify_identity_votes_given_proof(&proof_vec, &contract_lookup_fn, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert to JS object with hex keys
    let js_obj = Object::new();
    for (identifier, resource_vote) in votes_map {
        let hex_key = hex::encode(identifier.as_bytes());

        // Serialize resource vote to CBOR
        let vote_bytes = ciborium::ser::into_vec(&resource_vote)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize vote: {:?}", e)))?;
        let vote_uint8 = Uint8Array::from(&vote_bytes[..]);

        Reflect::set(&js_obj, &JsValue::from_str(&hex_key), &vote_uint8)
            .map_err(|_| JsValue::from_str("Failed to set vote in result object"))?;
    }

    Ok(VerifyIdentityVotesGivenProofResult {
        root_hash: root_hash.to_vec(),
        votes: js_obj.into(),
    })
}

// Helper function to create contract lookup function from JS object
fn create_contract_lookup_fn(contract_lookup: &JsValue) -> Result<ContractLookupFn, JsValue> {
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
        let contract: DataContract = ciborium::de::from_reader(&contract_bytes[..])
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;

        let identifier = contract.id();
        contracts_map.insert(identifier, Arc::new(contract));
    }

    let lookup_fn: ContractLookupFn =
        Arc::new(move |id: &Identifier| contracts_map.get(id).cloned());

    Ok(lookup_fn)
}
