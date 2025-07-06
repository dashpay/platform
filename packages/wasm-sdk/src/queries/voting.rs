use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::{Identifier, FetchMany};
use dash_sdk::dpp::platform_value::Value;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceResponse {
    contested_resource_id: String,
    value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContestedResourcesResponse {
    contested_resources: Vec<ContestedResourceResponse>,
    finished_results: bool,
}

// Simplified query structure for VotePollsByDocumentTypeQuery
#[derive(Debug, Clone)]
struct VotePollsByDocumentTypeQuery {
    contract_id: Identifier,
    document_type_name: String,
    index_name: String,
    start_index_values: Vec<Value>,
    end_index_values: Vec<Value>,
    start_at_value: Option<(Value, bool)>,
    limit: Option<u16>,
    order_ascending: bool,
}

#[wasm_bindgen]
pub async fn get_contested_resources(
    sdk: &WasmSdk,
    document_type_name: &str,
    data_contract_id: &str,
    index_name: &str,
    result_type: &str,
    _allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    start_at_value: Option<Vec<u8>>,
    limit: Option<u32>,
    _offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::ContestedResource;
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse start_at_value if provided
    let start_at = if let Some(bytes) = start_at_value {
        // Convert bytes to string and create a Value
        let value_str = String::from_utf8(bytes)
            .map_err(|e| JsError::new(&format!("Invalid UTF-8 in start_at_value: {}", e)))?;
        Some((Value::Text(value_str), true)) // inclusive by default
    } else {
        None
    };
    
    // Create the query - this is a simplified version
    let query = VotePollsByDocumentTypeQuery {
        contract_id,
        document_type_name: document_type_name.to_string(),
        index_name: index_name.to_string(),
        start_index_values: vec![], // Would need to be parsed from result_type or other params
        end_index_values: vec![],
        start_at_value: start_at,
        limit: limit.map(|l| l as u16),
        order_ascending: order_ascending.unwrap_or(true),
    };
    
    // For now, return an error since we can't create the proper query type
    // The SDK expects drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery
    // which is not exposed in WASM
    Err(JsError::new("getContestedResources requires drive query types that are not exposed in the WASM SDK. This query needs the VotePollsByDocumentTypeQuery from the drive crate."))
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContenderResponse {
    identifier: String,
    vote_count: Option<u32>,
    document: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceVoteStateResponse {
    contenders: Vec<ContenderResponse>,
    abstain_vote_tally: Option<u32>,
    lock_vote_tally: Option<u32>,
    finished_vote_info: Option<serde_json::Value>,
}

#[wasm_bindgen]
pub async fn get_contested_resource_vote_state(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    result_type: &str,
    allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    start_at_identifier_info: Option<String>,
    count: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::Contenders;
    use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // This query requires ContestedDocumentVotePollDriveQuery from drive crate
    // which is not exposed in WASM
    Err(JsError::new("getContestedResourceVoteState requires drive query types that are not exposed in the WASM SDK. This query needs the ContestedDocumentVotePollDriveQuery from the drive crate."))
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VoterResponse {
    identifier: String,
    voting_power: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VotersResponse {
    voters: Vec<VoterResponse>,
    finished_results: bool,
}

#[wasm_bindgen]
pub async fn get_contested_resource_voters_for_identity(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    contestant_id: &str,
    start_at_identifier_info: Option<String>,
    count: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::{Voter, Voters};
    
    // Parse IDs
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let contestant_identifier = Identifier::from_string(
        contestant_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // This query requires ContestedDocumentVotePollVotesDriveQuery from drive crate
    Err(JsError::new("getContestedResourceVotersForIdentity requires drive query types that are not exposed in the WASM SDK. This query needs the ContestedDocumentVotePollVotesDriveQuery from the drive crate."))
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ResourceVoteResponse {
    vote_poll_id: String,
    vote_choice: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ResourceVotesByIdentityResponse {
    votes: Vec<ResourceVoteResponse>,
    finished_results: bool,
}

#[wasm_bindgen]
pub async fn get_contested_resource_identity_votes(
    sdk: &WasmSdk,
    identity_id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::ResourceVotesByIdentity;
    use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
    
    // Parse identity ID
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // This query requires ContestedResourceVotesGivenByIdentityQuery from drive crate
    Err(JsError::new("getContestedResourceIdentityVotes requires drive query types that are not exposed in the WASM SDK. This query needs the ContestedResourceVotesGivenByIdentityQuery from the drive crate."))
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VotePollResponse {
    vote_poll_id: String,
    timestamp: u64,
    serialized_vote_poll: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VotePollsGroupedByTimestampResponse {
    vote_polls_by_timestamps: BTreeMap<u64, Vec<VotePollResponse>>,
    finished_results: bool,
}

#[wasm_bindgen]
pub async fn get_vote_polls_by_end_date(
    sdk: &WasmSdk,
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
    limit: Option<u32>,
    offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::voting::vote_polls::VotePoll;
    use drive_proof_verifier::types::VotePollsGroupedByTimestamp;
    use dash_sdk::dpp::prelude::TimestampMillis;
    
    // This query requires VotePollsByEndDateDriveQuery from drive crate
    Err(JsError::new("getVotePollsByEndDate requires drive query types that are not exposed in the WASM SDK. This query needs the VotePollsByEndDateDriveQuery from the drive crate."))
}