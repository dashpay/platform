use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::query::vote_poll_contestant_votes_query::ResolvedContestedDocumentVotePollVotesDriveQuery;
use drive::verify::RootHash;
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
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
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
    contestant_id: &Uint8Array,
    contested_document_resource_vote_poll_bytes: &Uint8Array,
    start_at: Option<Uint8Array>,
    limit: Option<u16>,
    order_ascending: bool,
    platform_version_number: u32,
) -> Result<VerifyVotePollVotesProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Deserialize the data contract
    let contract: DataContract = ciborium::de::from_reader(&contract_cbor.to_vec()[..])
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;
    let contract_arc = Arc::new(contract);

    // Parse the contestant ID
    let contestant_id_identifier = Identifier::from_bytes(&contestant_id.to_vec())
        .map_err(|e| JsValue::from_str(&format!("Invalid contestant ID: {:?}", e)))?;

    // Parse the contested document resource vote poll identifier
    let contested_document_resource_vote_poll =
        Identifier::from_bytes(&contested_document_resource_vote_poll_bytes.to_vec())
            .map_err(|e| JsValue::from_str(&format!("Invalid vote poll identifier: {:?}", e)))?;

    // Parse start_at if provided
    let start_at_identifier = start_at
        .map(|s| {
            Identifier::from_bytes(&s.to_vec())
                .map_err(|e| JsValue::from_str(&format!("Invalid start_at identifier: {:?}", e)))
        })
        .transpose()?;

    // Create the resolved query
    let query = ResolvedContestedDocumentVotePollVotesDriveQuery {
        contract: &contract_arc,
        document_type_name,
        contestant_id: contestant_id_identifier,
        contested_document_resource_vote_poll,
        start_at: start_at_identifier.as_ref(),
        limit,
        order_ascending,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, votes_vec) = query
        .verify_vote_poll_votes_proof(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert identifiers to JS array
    let js_array = Array::new();
    for identifier in votes_vec {
        let id_uint8 = Uint8Array::from(identifier.as_bytes());
        js_array.push(&id_uint8);
    }

    Ok(VerifyVotePollVotesProofResult {
        root_hash: root_hash.to_vec(),
        votes: js_array,
    })
}
