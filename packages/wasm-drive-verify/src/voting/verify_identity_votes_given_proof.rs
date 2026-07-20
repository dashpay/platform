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
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct ContestedResourceVotesGivenByIdentityWireQuery {
    identity_id: Vec<u8>,
    #[serde(default)]
    offset: Option<u16>,
    #[serde(default)]
    limit: Option<u16>,
    #[serde(default)]
    start_at: Option<([u8; 32], bool)>,
    #[serde(default = "default_order_ascending")]
    order_ascending: bool,
}

fn default_order_ascending() -> bool {
    true
}

fn deserialize_contested_resource_votes_query_bytes(
    query_cbor: &[u8],
) -> Result<ContestedResourceVotesGivenByIdentityQuery, String> {
    let query: ContestedResourceVotesGivenByIdentityWireQuery =
        ciborium::de::from_reader(query_cbor)
            .map_err(|e| format!("Failed to deserialize query: {e:?}"))?;
    let identity_id = Identifier::from_bytes(&query.identity_id)
        .map_err(|e| format!("Invalid identity_id: {e:?}"))?;

    Ok(ContestedResourceVotesGivenByIdentityQuery {
        identity_id,
        offset: query.offset,
        limit: query.limit,
        start_at: query.start_at,
        order_ascending: query.order_ascending,
    })
}

fn deserialize_contested_resource_votes_query(
    query_cbor: &Uint8Array,
) -> Result<ContestedResourceVotesGivenByIdentityQuery, JsValue> {
    deserialize_contested_resource_votes_query_bytes(&query_cbor.to_vec())
        .map_err(|e| JsValue::from_str(&e))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn encode_query(value: &Value) -> Vec<u8> {
        let mut bytes = Vec::new();
        ciborium::into_writer(value, &mut bytes).expect("encode query");
        bytes
    }

    fn valid_query() -> Value {
        json!({
            "identity_id": vec![1u8; 32],
            "offset": 7,
            "limit": 11,
            "start_at": [vec![2u8; 32], true],
            "order_ascending": false
        })
    }

    #[test]
    fn query_decoder_preserves_all_proof_critical_fields() {
        let query = deserialize_contested_resource_votes_query_bytes(&encode_query(&valid_query()))
            .expect("valid query");

        assert_eq!(query.identity_id, Identifier::from([1u8; 32]));
        assert_eq!(query.offset, Some(7));
        assert_eq!(query.limit, Some(11));
        assert_eq!(query.start_at, Some(([2u8; 32], true)));
        assert!(!query.order_ascending);
    }

    #[test]
    fn query_decoder_rejects_out_of_range_identity_bytes() {
        let mut query = valid_query();
        query["identity_id"][0] = json!(256);

        assert!(deserialize_contested_resource_votes_query_bytes(&encode_query(&query)).is_err());
    }

    #[test]
    fn query_decoder_rejects_out_of_range_pagination_values() {
        for field in ["offset", "limit"] {
            let mut query = valid_query();
            query[field] = json!(u16::MAX as u32 + 1);

            assert!(
                deserialize_contested_resource_votes_query_bytes(&encode_query(&query)).is_err(),
                "{field} should be rejected"
            );
        }
    }

    #[test]
    fn query_decoder_accepts_pagination_boundaries() {
        for value in [0, 1, u16::MAX] {
            let mut query = valid_query();
            query["offset"] = json!(value);
            query["limit"] = json!(value);

            let decoded = deserialize_contested_resource_votes_query_bytes(&encode_query(&query))
                .expect("pagination value should fit");
            assert_eq!(decoded.offset, Some(value));
            assert_eq!(decoded.limit, Some(value));
        }
    }

    #[test]
    fn query_decoder_rejects_out_of_range_start_at_bytes() {
        let mut query = valid_query();
        query["start_at"][0][31] = json!(256);

        assert!(deserialize_contested_resource_votes_query_bytes(&encode_query(&query)).is_err());
    }

    #[test]
    fn query_decoder_rejects_malformed_present_optional_fields() {
        for (field, malformed) in [
            ("offset", json!("1")),
            ("limit", json!(-1)),
            ("start_at", json!([vec![0u8; 31], true])),
        ] {
            let mut query = valid_query();
            query[field] = malformed;

            assert!(
                deserialize_contested_resource_votes_query_bytes(&encode_query(&query)).is_err(),
                "{field} should be rejected"
            );
        }
    }

    #[test]
    fn query_decoder_defaults_only_absent_optional_fields() {
        let query = json!({ "identity_id": vec![1u8; 32] });
        let decoded = deserialize_contested_resource_votes_query_bytes(&encode_query(&query))
            .expect("minimal query");

        assert_eq!(decoded.offset, None);
        assert_eq!(decoded.limit, None);
        assert_eq!(decoded.start_at, None);
        assert!(decoded.order_ascending);
    }
}
