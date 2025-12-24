use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use js_sys::{Array, Uint8Array};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyVotePollVotesProofResult {
    root_hash: Vec<u8>,
    votes: Array,
}

#[wasm_bindgen]
impl VerifyVotePollVotesProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn votes(&self) -> Array {
        self.votes.clone()
    }
}

#[wasm_bindgen(js_name = "verifyVotePollVotesProof")]
pub fn verify_vote_poll_votes_proof(
    proof: &Uint8Array,
    contract_cbor: &Uint8Array,
    document_type_name: &str,
    index_name: &str,
    contestant_id: &Uint8Array,
    contested_document_resource_vote_poll_bytes: &Uint8Array,
    start_at: Option<Uint8Array>,
    limit: Option<u16>,
    order_ascending: bool,
    platform_version_number: u32,
) -> Result<VerifyVotePollVotesProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse the data contract from CBOR
    let contract_bytes = contract_cbor.to_vec();
    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let contract = DataContract::versioned_deserialize(&contract_bytes, true, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;
    let contract_arc = Arc::new(contract);

    // Parse the contestant ID
    let contestant_id_identifier = Identifier::from_bytes(&contestant_id.to_vec())
        .map_err(|e| JsValue::from_str(&format!("Invalid contestant ID: {:?}", e)))?;

    // Parse the contested document resource vote poll identifier
    let contested_vote_poll_id =
        Identifier::from_bytes(&contested_document_resource_vote_poll_bytes.to_vec())
            .map_err(|e| JsValue::from_str(&format!("Invalid vote poll identifier: {:?}", e)))?;

    // Parse start_at if provided
    let start_at_identifier = start_at
        .map(|s| {
            Identifier::from_bytes(&s.to_vec())
                .map_err(|e| JsValue::from_str(&format!("Invalid start_at identifier: {:?}", e)))
        })
        .transpose()?;

    // Create the query
    let query = ContestedDocumentVotePollVotesDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id: contract_arc.id(),
            document_type_name: document_type_name.to_string(),
            index_name: index_name.to_string(),
            // Use the provided vote-poll ID as the index value
            index_values: vec![contested_vote_poll_id.into()],
        },
        contestant_id: contestant_id_identifier,
        offset: None,
        limit,
        start_at: start_at_identifier.map(|id| (id.to_buffer(), order_ascending)),
        order_ascending,
    };

    let contract_lookup =
        |_: &Identifier| -> Result<Option<Arc<DataContract>>, drive::error::Error> {
            Ok(Some(contract_arc.clone()))
        };

    let resolved_query = query
        .resolve_with_known_contracts_provider(&Box::new(contract_lookup))
        .map_err(|e| JsValue::from_str(&format!("Failed to resolve query: {:?}", e)))?;

    let (root_hash, votes_vec) = resolved_query
        .verify_vote_poll_votes_proof(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert identifiers to JS array
    let js_array = Array::new();
    for identifier in votes_vec {
        let id_bytes = identifier.as_bytes();
        let id_uint8 = Uint8Array::from(&id_bytes[..]);
        js_array.push(&id_uint8);
    }

    Ok(VerifyVotePollVotesProofResult {
        root_hash: root_hash.to_vec(),
        votes: js_array,
    })
}
