use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::Serialize;
use dash_sdk::platform::Identifier;
use dash_sdk::dpp::platform_value::Value;
use dapi_grpc::platform::v0::{
    GetContestedResourcesRequest, GetContestedResourceVoteStateRequest,
    GetContestedResourceVotersForIdentityRequest, GetContestedResourceIdentityVotesRequest,
    GetVotePollsByEndDateRequest,
    get_contested_resources_request::{self, GetContestedResourcesRequestV0},
    get_contested_resource_vote_state_request::{self, GetContestedResourceVoteStateRequestV0},
    get_contested_resource_voters_for_identity_request::{self, GetContestedResourceVotersForIdentityRequestV0},
    get_contested_resource_identity_votes_request::{self, GetContestedResourceIdentityVotesRequestV0},
    get_vote_polls_by_end_date_request::{self, GetVotePollsByEndDateRequestV0},
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_sdk::RequestSettings;
use rs_dapi_client::DapiRequestExecutor;

// Standard bincode configuration used by Platform
use dash_sdk::dpp::bincode;
const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();



#[wasm_bindgen]
pub async fn get_contested_resources(
    sdk: &WasmSdk,
    document_type_name: &str,
    data_contract_id: &str,
    index_name: &str,
    _result_type: &str,
    _allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    start_at_value: Option<Vec<u8>>,
    limit: Option<u32>,
    _offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse result_type to get start_index_values
    // The result_type parameter actually specifies what we want to query
    // For contested domain names in DPNS, we query at the "dash" parent domain level
    let start_index_values = if index_name == "parentNameAndLabel" && document_type_name == "domain" {
        // For DPNS domains, start at the parent domain level (e.g., "dash")
        vec![] // Empty to get all contested resources at any parent domain
    } else {
        // For other types, may need different index values
        vec![]
    };
    
    // Create start_at_value_info if provided
    let start_at_value_info = start_at_value.map(|bytes| {
        get_contested_resources_request::get_contested_resources_request_v0::StartAtValueInfo {
            start_value: bytes,
            start_value_included: true,
        }
    });
    
    // Create the gRPC request directly
    let request = GetContestedResourcesRequest {
        version: Some(get_contested_resources_request::Version::V0(
            GetContestedResourcesRequestV0 {
                contract_id: contract_id.to_vec(),
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                start_index_values,
                end_index_values: vec![],
                start_at_value_info,
                count: limit,
                order_ascending: order_ascending.unwrap_or(true),
                prove: sdk.prove(),
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resources: {}", e)))?;
    
    // For now, return a simple response structure
    // The actual response parsing would require the ContestedResource type
    let result = serde_json::json!({
        "contestedResources": [],
        "metadata": {
            "height": response.inner.metadata().ok().map(|m| m.height),
            "coreChainLockedHeight": response.inner.metadata().ok().map(|m| m.core_chain_locked_height),
            "timeMs": response.inner.metadata().ok().map(|m| m.time_ms),
            "protocolVersion": response.inner.metadata().ok().map(|m| m.protocol_version),
        }
    });
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    result.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_contested_resource_voters_for_identity(
    sdk: &WasmSdk,
    contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    index_values: Vec<JsValue>,
    contestant_id: &str,
    start_at_voter_info: Option<String>,
    limit: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse contestant ID
    let contestant_id = Identifier::from_string(
        contestant_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Convert JsValue index values to Vec<Vec<u8>> using bincode serialization
    let mut index_values_bytes: Vec<Vec<u8>> = Vec::new();
    for value in index_values {
        if let Some(s) = value.as_string() {
            // Create a platform Value from the string
            let platform_value = Value::Text(s);
            // Serialize using bincode
            let serialized = bincode::encode_to_vec(&platform_value, BINCODE_CONFIG)
                .map_err(|e| JsError::new(&format!("Failed to serialize index value: {}", e)))?;
            index_values_bytes.push(serialized);
        } else {
            return Err(JsError::new("Index values must be strings"));
        }
    }
    
    // Parse start_at_voter_info if provided
    let start_at_identifier_info = if let Some(info_str) = start_at_voter_info {
        let info: serde_json::Value = serde_json::from_str(&info_str)
            .map_err(|e| JsError::new(&format!("Invalid start_at_voter_info JSON: {}", e)))?;
        
        if let (Some(start_id), Some(included)) = (info.get("startIdentifier"), info.get("startIdentifierIncluded")) {
            let start_identifier = start_id.as_str()
                .ok_or_else(|| JsError::new("startIdentifier must be a string"))?
                .as_bytes()
                .to_vec();
            let start_identifier_included = included.as_bool().unwrap_or(true);
            
            Some(get_contested_resource_voters_for_identity_request::get_contested_resource_voters_for_identity_request_v0::StartAtIdentifierInfo {
                start_identifier,
                start_identifier_included,
            })
        } else {
            None
        }
    } else {
        None
    };
    
    // Create the gRPC request
    let request = GetContestedResourceVotersForIdentityRequest {
        version: Some(get_contested_resource_voters_for_identity_request::Version::V0(
            GetContestedResourceVotersForIdentityRequestV0 {
                contract_id: contract_id.to_vec(),
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                index_values: index_values_bytes,
                contestant_id: contestant_id.to_vec(),
                start_at_identifier_info,
                count: limit,
                order_ascending: order_ascending.unwrap_or(true),
                prove: sdk.prove(),
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resource voters: {}", e)))?;
    
    // For now, return a simple response structure
    let result = serde_json::json!({
        "voters": [],
        "metadata": {
            "height": response.inner.metadata().ok().map(|m| m.height),
            "coreChainLockedHeight": response.inner.metadata().ok().map(|m| m.core_chain_locked_height),
            "timeMs": response.inner.metadata().ok().map(|m| m.time_ms),
            "protocolVersion": response.inner.metadata().ok().map(|m| m.protocol_version),
        }
    });
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    result.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_contested_resource_identity_votes(
    sdk: &WasmSdk,
    identity_id: &str,
    limit: Option<u32>,
    start_at_vote_poll_id_info: Option<String>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    // TODO: Implement get_contested_resource_identity_votes
    // This function should return all votes made by a specific identity
    let _ = (sdk, identity_id, limit, start_at_vote_poll_id_info, order_ascending);
    
    // Return empty result for now
    let result = serde_json::json!({
        "votes": [],
        "metadata": {}
    });
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_vote_polls_by_end_date(
    sdk: &WasmSdk,
    start_time_info: Option<String>,
    end_time_info: Option<String>,
    limit: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    // TODO: Implement get_vote_polls_by_end_date
    // This function should return vote polls filtered by end date
    let _ = (sdk, start_time_info, end_time_info, limit, order_ascending);
    
    // Return empty result for now
    let result = serde_json::json!({
        "votePolls": [],
        "metadata": {}
    });
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Proof info versions for voting queries

#[wasm_bindgen]
pub async fn get_contested_resources_with_proof_info(
    sdk: &WasmSdk,
    document_type_name: &str,
    data_contract_id: &str,
    index_name: &str,
    _result_type: &str,
    _allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    start_at_value: Option<Vec<u8>>,
    limit: Option<u32>,
    _offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use crate::queries::ProofMetadataResponse;
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse result_type to get start_index_values
    // The result_type parameter actually specifies what we want to query
    // For contested domain names in DPNS, we query at the "dash" parent domain level
    let start_index_values = if index_name == "parentNameAndLabel" && document_type_name == "domain" {
        // For DPNS domains, start at the parent domain level (e.g., "dash")
        vec![] // Empty to get all contested resources at any parent domain
    } else {
        // For other types, may need different index values
        vec![]
    };
    
    // Create start_at_value_info if provided
    let start_at_value_info = start_at_value.map(|bytes| {
        get_contested_resources_request::get_contested_resources_request_v0::StartAtValueInfo {
            start_value: bytes,
            start_value_included: true,
        }
    });
    
    // Create the gRPC request directly - force prove=true for proof info
    let request = GetContestedResourcesRequest {
        version: Some(get_contested_resources_request::Version::V0(
            GetContestedResourcesRequestV0 {
                contract_id: contract_id.to_vec(),
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                start_index_values,
                end_index_values: vec![],
                start_at_value_info,
                count: limit,
                order_ascending: order_ascending.unwrap_or(true),
                prove: true, // Always true for proof info version
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resources with proof: {}", e)))?;
    
    // Extract metadata and proof from response
    let metadata = response.inner.metadata()
        .map_err(|e| JsError::new(&format!("Failed to get metadata: {:?}", e)))?;
    
    let proof = response.inner.proof()
        .map_err(|e| JsError::new(&format!("Failed to get proof: {:?}", e)))?;
    
    // For now, return a simple response structure
    let data = serde_json::json!({
        "contestedResources": []
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.clone().into(),
        proof: proof.clone().into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_contested_resource_vote_state_with_proof_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    index_values: Vec<JsValue>,
    _result_type: &str,
    allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    start_at_identifier_info: Option<String>,
    count: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use crate::queries::ProofMetadataResponse;
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse start_at_identifier_info if provided
    let start_at_identifier_info = if let Some(info_str) = start_at_identifier_info {
        let info: serde_json::Value = serde_json::from_str(&info_str)
            .map_err(|e| JsError::new(&format!("Invalid start_at_identifier_info JSON: {}", e)))?;
        
        if let (Some(start_id), Some(included)) = (info.get("startIdentifier"), info.get("startIdentifierIncluded")) {
            let start_identifier = start_id.as_str()
                .ok_or_else(|| JsError::new("startIdentifier must be a string"))?
                .as_bytes()
                .to_vec();
            let start_identifier_included = included.as_bool().unwrap_or(true);
            
            Some(get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo {
                start_identifier,
                start_identifier_included,
            })
        } else {
            None
        }
    } else {
        None
    };
    
    // Convert JsValue index values to Vec<Vec<u8>> using bincode serialization
    let mut index_values_bytes: Vec<Vec<u8>> = Vec::new();
    for value in index_values {
        if let Some(s) = value.as_string() {
            // Create a platform Value from the string
            let platform_value = Value::Text(s);
            // Serialize using bincode
            let serialized = bincode::encode_to_vec(&platform_value, BINCODE_CONFIG)
                .map_err(|e| JsError::new(&format!("Failed to serialize index value: {}", e)))?;
            index_values_bytes.push(serialized);
        } else {
            return Err(JsError::new("Index values must be strings"));
        }
    }
    
    // Create the gRPC request directly - force prove=true
    let request = GetContestedResourceVoteStateRequest {
        version: Some(get_contested_resource_vote_state_request::Version::V0(
            GetContestedResourceVoteStateRequestV0 {
                contract_id: contract_id.to_vec(),
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                index_values: index_values_bytes,
                result_type: if allow_include_locked_and_abstaining_vote_tally.unwrap_or(false) { 0 } else { 1 },
                allow_include_locked_and_abstaining_vote_tally: allow_include_locked_and_abstaining_vote_tally.unwrap_or(false),
                start_at_identifier_info,
                count,
                prove: true, // Always true for proof info version
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resource vote state with proof: {}", e)))?;
    
    // Extract metadata and proof from response
    let metadata = response.inner.metadata()
        .map_err(|e| JsError::new(&format!("Failed to get metadata: {:?}", e)))?;
    
    let proof = response.inner.proof()
        .map_err(|e| JsError::new(&format!("Failed to get proof: {:?}", e)))?;
    
    // Return a simple response structure
    let data = serde_json::json!({
        "contenders": [],
        "abstainVoteTally": null,
        "lockVoteTally": null,
        "finishedVoteInfo": null
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.clone().into(),
        proof: proof.clone().into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_contested_resource_voters_for_identity_with_proof_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    index_values: Vec<JsValue>,
    contestant_id: &str,
    start_at_identifier_info: Option<String>,
    count: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use crate::queries::ProofMetadataResponse;
    
    // Parse IDs
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let contestant_identifier = Identifier::from_string(
        contestant_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Convert JsValue index values to Vec<Vec<u8>> using bincode serialization
    let mut index_values_bytes: Vec<Vec<u8>> = Vec::new();
    for value in index_values {
        if let Some(s) = value.as_string() {
            // Create a platform Value from the string
            let platform_value = Value::Text(s);
            // Serialize using bincode
            let serialized = bincode::encode_to_vec(&platform_value, BINCODE_CONFIG)
                .map_err(|e| JsError::new(&format!("Failed to serialize index value: {}", e)))?;
            index_values_bytes.push(serialized);
        } else {
            return Err(JsError::new("Index values must be strings"));
        }
    }
    
    // Parse start_at_identifier_info if provided
    let start_at_identifier_info = if let Some(info_str) = start_at_identifier_info {
        let info: serde_json::Value = serde_json::from_str(&info_str)
            .map_err(|e| JsError::new(&format!("Invalid start_at_identifier_info JSON: {}", e)))?;
        
        if let (Some(start_id), Some(included)) = (info.get("startIdentifier"), info.get("startIdentifierIncluded")) {
            let start_identifier = start_id.as_str()
                .ok_or_else(|| JsError::new("startIdentifier must be a string"))?
                .as_bytes()
                .to_vec();
            let start_identifier_included = included.as_bool().unwrap_or(true);
            
            Some(get_contested_resource_voters_for_identity_request::get_contested_resource_voters_for_identity_request_v0::StartAtIdentifierInfo {
                start_identifier,
                start_identifier_included,
            })
        } else {
            None
        }
    } else {
        None
    };
    
    // Create the gRPC request directly - force prove=true
    let request = GetContestedResourceVotersForIdentityRequest {
        version: Some(get_contested_resource_voters_for_identity_request::Version::V0(
            GetContestedResourceVotersForIdentityRequestV0 {
                contract_id: contract_id.to_vec(),
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                index_values: index_values_bytes,
                contestant_id: contestant_identifier.to_vec(),
                start_at_identifier_info,
                count,
                order_ascending: order_ascending.unwrap_or(true),
                prove: true, // Always true for proof info version
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resource voters with proof: {}", e)))?;
    
    // Extract metadata and proof from response
    let metadata = response.inner.metadata()
        .map_err(|e| JsError::new(&format!("Failed to get metadata: {:?}", e)))?;
    
    let proof = response.inner.proof()
        .map_err(|e| JsError::new(&format!("Failed to get proof: {:?}", e)))?;
    
    // Return a simple response structure
    let data = serde_json::json!({
        "voters": [],
        "finishedResults": false
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.clone().into(),
        proof: proof.clone().into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_contested_resource_identity_votes_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use crate::queries::ProofMetadataResponse;
    
    // Parse identity ID
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Create the gRPC request directly - force prove=true
    let request = GetContestedResourceIdentityVotesRequest {
        version: Some(get_contested_resource_identity_votes_request::Version::V0(
            GetContestedResourceIdentityVotesRequestV0 {
                identity_id: identity_identifier.to_vec(),
                limit,
                offset,
                order_ascending: order_ascending.unwrap_or(true),
                start_at_vote_poll_id_info: None,
                prove: true, // Always true for proof info version
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resource identity votes with proof: {}", e)))?;
    
    // Extract metadata and proof from response
    let metadata = response.inner.metadata()
        .map_err(|e| JsError::new(&format!("Failed to get metadata: {:?}", e)))?;
    
    let proof = response.inner.proof()
        .map_err(|e| JsError::new(&format!("Failed to get proof: {:?}", e)))?;
    
    // Return a simple response structure
    let data = serde_json::json!({
        "votes": [],
        "finishedResults": false
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.clone().into(),
        proof: proof.clone().into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_vote_polls_by_end_date_with_proof_info(
    sdk: &WasmSdk,
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
    limit: Option<u32>,
    offset: Option<u32>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use crate::queries::ProofMetadataResponse;
    
    // Note: GetVotePollsByEndDateRequestV0 doesn't have start_at_poll_info, only offset
    
    // Create the gRPC request directly - force prove=true
    let request = GetVotePollsByEndDateRequest {
        version: Some(get_vote_polls_by_end_date_request::Version::V0(
            GetVotePollsByEndDateRequestV0 {
                start_time_info: start_time_ms.map(|ms| {
                    get_vote_polls_by_end_date_request::get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                        start_time_ms: ms,
                        start_time_included: true,
                    }
                }),
                end_time_info: end_time_ms.map(|ms| {
                    get_vote_polls_by_end_date_request::get_vote_polls_by_end_date_request_v0::EndAtTimeInfo {
                        end_time_ms: ms,
                        end_time_included: true,
                    }
                }),
                limit,
                offset,
                ascending: order_ascending.unwrap_or(true),
                prove: true, // Always true for proof info version
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get vote polls by end date with proof: {}", e)))?;
    
    // Extract metadata and proof from response
    let metadata = response.inner.metadata()
        .map_err(|e| JsError::new(&format!("Failed to get metadata: {:?}", e)))?;
    
    let proof = response.inner.proof()
        .map_err(|e| JsError::new(&format!("Failed to get proof: {:?}", e)))?;
    
    // Return a simple response structure
    let data = serde_json::json!({
        "votePollsByTimestamps": {},
        "finishedResults": false
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.clone().into(),
        proof: proof.clone().into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}



#[wasm_bindgen]
pub async fn get_contested_resource_vote_state(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type_name: &str,
    index_name: &str,
    index_values: Vec<JsValue>,
    _result_type: &str,
    allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    start_at_identifier_info: Option<String>,
    count: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse start_at_identifier_info if provided
    let start_at_identifier_info = if let Some(info_str) = start_at_identifier_info {
        let info: serde_json::Value = serde_json::from_str(&info_str)
            .map_err(|e| JsError::new(&format!("Invalid start_at_identifier_info JSON: {}", e)))?;
        
        if let (Some(start_id), Some(included)) = (info.get("startIdentifier"), info.get("startIdentifierIncluded")) {
            let start_identifier = start_id.as_str()
                .ok_or_else(|| JsError::new("startIdentifier must be a string"))?
                .as_bytes()
                .to_vec();
            let start_identifier_included = included.as_bool().unwrap_or(true);
            
            Some(get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo {
                start_identifier,
                start_identifier_included,
            })
        } else {
            None
        }
    } else {
        None
    };
    
    // Convert JsValue index values to Vec<Vec<u8>> using bincode serialization
    let mut index_values_bytes: Vec<Vec<u8>> = Vec::new();
    for value in index_values {
        if let Some(s) = value.as_string() {
            // Create a platform Value from the string
            let platform_value = Value::Text(s);
            // Serialize using bincode
            let serialized = bincode::encode_to_vec(&platform_value, BINCODE_CONFIG)
                .map_err(|e| JsError::new(&format!("Failed to serialize index value: {}", e)))?;
            index_values_bytes.push(serialized);
        } else {
            return Err(JsError::new("Index values must be strings"));
        }
    }
    
    // Create the gRPC request directly
    let request = GetContestedResourceVoteStateRequest {
        version: Some(get_contested_resource_vote_state_request::Version::V0(
            GetContestedResourceVoteStateRequestV0 {
                contract_id: contract_id.to_vec(),
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                index_values: index_values_bytes,
                result_type: if allow_include_locked_and_abstaining_vote_tally.unwrap_or(false) { 0 } else { 1 },
                allow_include_locked_and_abstaining_vote_tally: allow_include_locked_and_abstaining_vote_tally.unwrap_or(false),
                start_at_identifier_info,
                count,
                prove: sdk.prove(),
            },
        )),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to get contested resource vote state: {}", e)))?;
    
    // Return a simple response structure
    let result = serde_json::json!({
        "contenders": [],
        "abstainVoteTally": null,
        "lockVoteTally": null,
        "finishedVoteInfo": null,
        "metadata": {
            "height": response.inner.metadata().ok().map(|m| m.height),
            "coreChainLockedHeight": response.inner.metadata().ok().map(|m| m.core_chain_locked_height),
            "timeMs": response.inner.metadata().ok().map(|m| m.time_ms),
            "protocolVersion": response.inner.metadata().ok().map(|m| m.protocol_version),
        }
    });
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    result.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

