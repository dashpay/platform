use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::{Fetch, FetchMany, Identifier};
use dash_sdk::dpp::data_contract::group::Group;
use dash_sdk::dpp::data_contract::GroupContractPosition;
use dash_sdk::dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dash_sdk::platform::group_actions::{GroupQuery, GroupInfosQuery, GroupActionsQuery, GroupActionSignersQuery};
use dash_sdk::dpp::group::group_action::GroupAction;
use dash_sdk::dpp::group::group_action_status::GroupActionStatus;
use dash_sdk::dpp::data_contract::group::GroupMemberPower;
use std::collections::BTreeMap;

// Proof info functions are now included below

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfoResponse {
    members: BTreeMap<String, u32>,
    required_power: u32,
}

impl GroupInfoResponse {
    fn from_group(group: &Group) -> Self {
        let members = group.members()
            .iter()
            .map(|(id, power)| {
                (id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58), *power)
            })
            .collect();
        
        Self {
            members,
            required_power: group.required_power(),
        }
    }
}

#[wasm_bindgen]
pub async fn get_group_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    group_contract_position: u32,
) -> Result<JsValue, JsError> {
    // Parse data contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Create group query
    let query = GroupQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
    };
    
    // Fetch the group
    let group_result: Option<Group> = Group::fetch(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group: {}", e)))?;
    
    match group_result {
        Some(group) => {
            let response = GroupInfoResponse::from_group(&group);
            
            // Use json_compatible serializer to convert maps to objects
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response.serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        },
        None => Ok(JsValue::NULL),
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GroupMember {
    member_id: String,
    power: u32,
}

#[wasm_bindgen]
pub async fn get_group_members(
    sdk: &WasmSdk,
    data_contract_id: &str,
    group_contract_position: u32,
    member_ids: Option<Vec<String>>,
    start_at: Option<String>,
    limit: Option<u32>,
) -> Result<JsValue, JsError> {
    // Parse data contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Create group query
    let query = GroupQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
    };
    
    // Fetch the group
    let group_result: Option<Group> = Group::fetch(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group: {}", e)))?;
    
    match group_result {
        Some(group) => {
            let mut members: Vec<GroupMember> = Vec::new();
            
            // If specific member IDs are requested, filter by them
            if let Some(requested_ids) = member_ids {
                let requested_identifiers: Result<Vec<Identifier>, _> = requested_ids
                    .iter()
                    .map(|id| Identifier::from_string(
                        id,
                        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                    ))
                    .collect();
                let requested_identifiers = requested_identifiers?;
                
                for id in requested_identifiers {
                    if let Ok(power) = group.member_power(id) {
                        members.push(GroupMember {
                            member_id: id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                            power,
                        });
                    }
                }
            } else {
                // Return all members with pagination
                let all_members = group.members();
                let mut sorted_members: Vec<_> = all_members.iter().collect();
                sorted_members.sort_by_key(|(id, _)| *id);
                
                // Apply start_at if provided
                let start_index = if let Some(start_id) = start_at {
                    let start_identifier = Identifier::from_string(
                        &start_id,
                        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                    )?;
                    sorted_members.iter().position(|(id, _)| **id > start_identifier).unwrap_or(sorted_members.len())
                } else {
                    0
                };
                
                // Apply limit
                let end_index = if let Some(lim) = limit {
                    (start_index + lim as usize).min(sorted_members.len())
                } else {
                    sorted_members.len()
                };
                
                for (id, power) in &sorted_members[start_index..end_index] {
                    members.push(GroupMember {
                        member_id: (*id).to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                        power: **power,
                    });
                }
            }
            
            // Use json_compatible serializer to convert response
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            members.serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        },
        None => Ok(JsValue::NULL),
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IdentityGroupInfo {
    data_contract_id: String,
    group_contract_position: u32,
    role: String, // "member", "owner", or "moderator"
    power: Option<u32>, // Only for members
}

#[wasm_bindgen]
pub async fn get_identity_groups(
    sdk: &WasmSdk,
    identity_id: &str,
    member_data_contracts: Option<Vec<String>>,
    owner_data_contracts: Option<Vec<String>>,
    moderator_data_contracts: Option<Vec<String>>,
) -> Result<JsValue, JsError> {
    // Parse identity ID
    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let mut groups: Vec<IdentityGroupInfo> = Vec::new();
    
    // Check member data contracts
    if let Some(contracts) = member_data_contracts {
        for contract_id_str in contracts {
            let contract_id = Identifier::from_string(
                &contract_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )?;
            
            // Fetch all groups for this contract
            let query = GroupInfosQuery {
                contract_id,
                start_group_contract_position: None,
                limit: None,
            };
            
            let groups_result = Group::fetch_many(sdk.as_ref(), query)
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch groups: {}", e)))?;
            
            // Check each group for the identity
            for (position, group_opt) in groups_result {
                if let Some(group) = group_opt {
                    if let Ok(power) = group.member_power(id) {
                        groups.push(IdentityGroupInfo {
                            data_contract_id: contract_id_str.clone(),
                            group_contract_position: position as u32,
                            role: "member".to_string(),
                            power: Some(power),
                        });
                    }
                }
            }
        }
    }
    
    // Note: Owner and moderator roles would require additional contract queries
    // which are not yet implemented in the SDK. For now, return a warning.
    if owner_data_contracts.is_some() || moderator_data_contracts.is_some() {
        web_sys::console::warn_1(&JsValue::from_str(
            "Warning: Owner and moderator role queries are not yet implemented"
        ));
    }
    
    // Use json_compatible serializer to convert response
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    groups.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GroupsDataContractInfo {
    data_contract_id: String,
    groups: Vec<GroupContractPositionInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GroupContractPositionInfo {
    position: u32,
    group: GroupInfoResponse,
}

#[wasm_bindgen]
pub async fn get_group_infos(
    sdk: &WasmSdk,
    contract_id: &str,
    start_at_info: JsValue,
    count: Option<u32>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse start at info if provided
    let start_group_contract_position = if !start_at_info.is_null() && !start_at_info.is_undefined() {
        let info = serde_wasm_bindgen::from_value::<serde_json::Value>(start_at_info);
        match info {
            Ok(json) => {
                let position = json["position"].as_u64().ok_or_else(|| JsError::new("Invalid start position"))? as u32;
                let included = json["included"].as_bool().unwrap_or(false);
                Some((position as GroupContractPosition, included))
            }
            Err(_) => None
        }
    } else {
        None
    };
    
    // Create query
    let query = GroupInfosQuery {
        contract_id,
        start_group_contract_position,
        limit: count.map(|c| c as u16),
    };
    
    // Fetch groups
    let groups_result = Group::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch groups: {}", e)))?;
    
    // Convert result to response format
    let mut group_infos = Vec::new();
    for (position, group_opt) in groups_result {
        if let Some(group) = group_opt {
            let members: Vec<serde_json::Value> = group.members()
                .iter()
                .map(|(id, power)| {
                    serde_json::json!({
                        "memberId": id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                        "power": *power
                    })
                })
                .collect();
            
            group_infos.push(serde_json::json!({
                "groupContractPosition": position,
                "members": members,
                "groupRequiredPower": group.required_power()
            }));
        }
    }
    
    let response = serde_json::json!({
        "groupInfos": group_infos
    });
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_group_actions(
    sdk: &WasmSdk,
    contract_id: &str,
    group_contract_position: u32,
    status: &str,
    start_at_info: JsValue,
    count: Option<u32>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse status
    let status = match status {
        "ACTIVE" => GroupActionStatus::ActionActive,
        "CLOSED" => GroupActionStatus::ActionClosed,
        _ => return Err(JsError::new(&format!("Invalid status: {}. Must be ACTIVE or CLOSED", status))),
    };
    
    // Parse start action ID if provided
    let start_at_action_id = if !start_at_info.is_null() && !start_at_info.is_undefined() {
        let info = serde_wasm_bindgen::from_value::<serde_json::Value>(start_at_info);
        match info {
            Ok(json) => {
                let action_id = json["actionId"].as_str().ok_or_else(|| JsError::new("Invalid action ID"))?;
                let included = json["included"].as_bool().unwrap_or(false);
                Some((
                    Identifier::from_string(
                        action_id,
                        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                    )?,
                    included
                ))
            }
            Err(_) => None
        }
    } else {
        None
    };
    
    // Create query
    let query = GroupActionsQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
        status,
        start_at_action_id,
        limit: count.map(|c| c as u16),
    };
    
    // Fetch actions
    let actions_result = GroupAction::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group actions: {}", e)))?;
    
    // Convert result to response format
    let mut group_actions = Vec::new();
    for (action_id, action_opt) in actions_result {
        if let Some(_action) = action_opt {
            // For now, just return the action ID
            // The full action structure requires custom serialization
            group_actions.push(serde_json::json!({
                "actionId": action_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                // TODO: Serialize the full action event structure
            }));
        }
    }
    
    let response = serde_json::json!({
        "groupActions": group_actions
    });
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_group_action_signers(
    sdk: &WasmSdk,
    contract_id: &str,
    group_contract_position: u32,
    status: &str,
    action_id: &str,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse action ID
    let action_id = Identifier::from_string(
        action_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse status
    let status = match status {
        "ACTIVE" => GroupActionStatus::ActionActive,
        "CLOSED" => GroupActionStatus::ActionClosed,
        _ => return Err(JsError::new(&format!("Invalid status: {}. Must be ACTIVE or CLOSED", status))),
    };
    
    // Create query
    let query = GroupActionSignersQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
        status,
        action_id,
    };
    
    // Fetch signers
    let signers_result = GroupMemberPower::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group action signers: {}", e)))?;
    
    // Convert result to response format
    let mut signers = Vec::new();
    for (signer_id, power_opt) in signers_result {
        if let Some(power) = power_opt {
            signers.push(serde_json::json!({
                "signerId": signer_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                "power": power
            }));
        }
    }
    
    let response = serde_json::json!({
        "signers": signers
    });
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_groups_data_contracts(
    sdk: &WasmSdk,
    data_contract_ids: Vec<String>,
) -> Result<JsValue, JsError> {
    let mut results: Vec<GroupsDataContractInfo> = Vec::new();
    
    for contract_id_str in data_contract_ids {
        let contract_id = Identifier::from_string(
            &contract_id_str,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        
        // Fetch all groups for this contract
        let query = GroupInfosQuery {
            contract_id,
            start_group_contract_position: None,
            limit: None,
        };
        
        let groups_result = Group::fetch_many(sdk.as_ref(), query)
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch groups for contract {}: {}", contract_id_str, e)))?;
        
        let mut groups: Vec<GroupContractPositionInfo> = Vec::new();
        
        for (position, group_opt) in groups_result {
            if let Some(group) = group_opt {
                groups.push(GroupContractPositionInfo {
                    position: position as u32,
                    group: GroupInfoResponse::from_group(&group),
                });
            }
        }
        
        results.push(GroupsDataContractInfo {
            data_contract_id: contract_id_str,
            groups,
        });
    }
    
    // Use json_compatible serializer to convert response
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    results.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Proof versions for group queries

#[wasm_bindgen]
pub async fn get_group_info_with_proof_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    group_contract_position: u32,
) -> Result<JsValue, JsError> {
    use crate::queries::{ProofMetadataResponse, ResponseMetadata, ProofInfo};
    
    // Parse data contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Create group query
    let query = GroupQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
    };
    
    // Fetch group with proof
    let (group_result, metadata, proof) = Group::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group with proof: {}", e)))?;
    
    let data = group_result.map(|group| GroupInfoResponse::from_group(&group));
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_group_infos_with_proof_info(
    sdk: &WasmSdk,
    contract_id: &str,
    start_at_info: JsValue,
    count: Option<u32>,
) -> Result<JsValue, JsError> {
    use crate::queries::{ProofMetadataResponse, ResponseMetadata, ProofInfo};
    
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse start at info if provided
    let start_group_contract_position = if !start_at_info.is_null() && !start_at_info.is_undefined() {
        let info = serde_wasm_bindgen::from_value::<serde_json::Value>(start_at_info);
        match info {
            Ok(json) => {
                let position = json["position"].as_u64().ok_or_else(|| JsError::new("Invalid start position"))? as u32;
                let included = json["included"].as_bool().unwrap_or(false);
                Some((position as GroupContractPosition, included))
            }
            Err(_) => None
        }
    } else {
        None
    };
    
    // Create query
    let query = GroupInfosQuery {
        contract_id,
        start_group_contract_position,
        limit: count.map(|c| c as u16),
    };
    
    // Fetch groups with proof
    let (groups_result, metadata, proof) = Group::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch groups with proof: {}", e)))?;
    
    // Convert result to response format
    let mut group_infos = Vec::new();
    for (position, group_opt) in groups_result {
        if let Some(group) = group_opt {
            group_infos.push(GroupContractPositionInfo {
                position: position as u32,
                group: GroupInfoResponse::from_group(&group),
            });
        }
    }
    
    let data = serde_json::json!({
        "groupInfos": group_infos
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Additional proof info versions for remaining group queries
use crate::queries::{ProofMetadataResponse, ResponseMetadata, ProofInfo};

#[wasm_bindgen]
pub async fn get_group_members_with_proof_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    group_contract_position: u32,
    member_ids: Option<Vec<String>>,
    start_at: Option<String>,
    limit: Option<u32>,
) -> Result<JsValue, JsError> {
    // Parse data contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Create group query
    let query = GroupQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
    };
    
    // Fetch the group with proof
    let (group_result, metadata, proof) = Group::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group with proof: {}", e)))?;
    
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct GroupMember {
        member_id: String,
        power: u32,
    }
    
    let data = match group_result {
        Some(group) => {
            let mut members: Vec<GroupMember> = Vec::new();
            
            // If specific member IDs are requested, filter by them
            if let Some(requested_ids) = member_ids {
                let requested_identifiers: Result<Vec<Identifier>, _> = requested_ids
                    .iter()
                    .map(|id| Identifier::from_string(
                        id,
                        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                    ))
                    .collect();
                let requested_identifiers = requested_identifiers?;
                
                for id in requested_identifiers {
                    if let Ok(power) = group.member_power(id) {
                        members.push(GroupMember {
                            member_id: id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                            power,
                        });
                    }
                }
            } else {
                // Return all members with pagination
                let all_members = group.members();
                let mut sorted_members: Vec<_> = all_members.iter().collect();
                sorted_members.sort_by_key(|(id, _)| *id);
                
                // Apply start_at if provided
                let start_index = if let Some(start_id) = start_at {
                    let start_identifier = Identifier::from_string(
                        &start_id,
                        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                    )?;
                    sorted_members.iter().position(|(id, _)| **id > start_identifier).unwrap_or(sorted_members.len())
                } else {
                    0
                };
                
                // Apply limit
                let end_index = if let Some(lim) = limit {
                    (start_index + lim as usize).min(sorted_members.len())
                } else {
                    sorted_members.len()
                };
                
                for (id, power) in &sorted_members[start_index..end_index] {
                    members.push(GroupMember {
                        member_id: (*id).to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                        power: **power,
                    });
                }
            }
            
            Some(members)
        },
        None => None,
    };
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_identity_groups_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    member_data_contracts: Option<Vec<String>>,
    owner_data_contracts: Option<Vec<String>>,
    moderator_data_contracts: Option<Vec<String>>,
) -> Result<JsValue, JsError> {
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct IdentityGroupInfo {
        data_contract_id: String,
        group_contract_position: u32,
        role: String, // "member", "owner", or "moderator"
        power: Option<u32>, // Only for members
    }
    
    // Parse identity ID
    let id = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    let mut groups: Vec<IdentityGroupInfo> = Vec::new();
    let mut combined_metadata: Option<ResponseMetadata> = None;
    let mut combined_proof: Option<ProofInfo> = None;
    
    // Check member data contracts
    if let Some(contracts) = member_data_contracts {
        for contract_id_str in contracts {
            let contract_id = Identifier::from_string(
                &contract_id_str,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )?;
            
            // Fetch all groups for this contract with proof
            let query = GroupInfosQuery {
                contract_id,
                start_group_contract_position: None,
                limit: None,
            };
            
            let (groups_result, metadata, proof) = Group::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
                .await
                .map_err(|e| JsError::new(&format!("Failed to fetch groups with proof: {}", e)))?;
            
            // Store first metadata and proof
            if combined_metadata.is_none() {
                combined_metadata = Some(metadata.into());
                combined_proof = Some(proof.into());
            }
            
            // Check each group for the identity
            for (position, group_opt) in groups_result {
                if let Some(group) = group_opt {
                    if let Ok(power) = group.member_power(id) {
                        groups.push(IdentityGroupInfo {
                            data_contract_id: contract_id_str.clone(),
                            group_contract_position: position as u32,
                            role: "member".to_string(),
                            power: Some(power),
                        });
                    }
                }
            }
        }
    }
    
    // Note: Owner and moderator roles would require additional contract queries
    // which are not yet implemented in the SDK. For now, return a warning.
    if owner_data_contracts.is_some() || moderator_data_contracts.is_some() {
        web_sys::console::warn_1(&JsValue::from_str(
            "Warning: Owner and moderator role queries are not yet implemented"
        ));
    }
    
    let response = ProofMetadataResponse {
        data: groups,
        metadata: combined_metadata.unwrap_or_else(|| ResponseMetadata {
            height: 0,
            core_chain_locked_height: 0,
            epoch: 0,
            time_ms: 0,
            protocol_version: 0,
            chain_id: String::new(),
        }),
        proof: combined_proof.unwrap_or_else(|| ProofInfo {
            grovedb_proof: String::new(),
            quorum_hash: String::new(),
            signature: String::new(),
            round: 0,
            block_id_hash: String::new(),
            quorum_type: 0,
        }),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_group_actions_with_proof_info(
    sdk: &WasmSdk,
    contract_id: &str,
    group_contract_position: u32,
    status: &str,
    start_at_info: JsValue,
    count: Option<u32>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse status
    let status = match status {
        "ACTIVE" => GroupActionStatus::ActionActive,
        "CLOSED" => GroupActionStatus::ActionClosed,
        _ => return Err(JsError::new(&format!("Invalid status: {}. Must be ACTIVE or CLOSED", status))),
    };
    
    // Parse start action ID if provided
    let start_at_action_id = if !start_at_info.is_null() && !start_at_info.is_undefined() {
        let info = serde_wasm_bindgen::from_value::<serde_json::Value>(start_at_info);
        match info {
            Ok(json) => {
                let action_id = json["actionId"].as_str().ok_or_else(|| JsError::new("Invalid action ID"))?;
                let included = json["included"].as_bool().unwrap_or(false);
                Some((
                    Identifier::from_string(
                        action_id,
                        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                    )?,
                    included
                ))
            }
            Err(_) => None
        }
    } else {
        None
    };
    
    // Create query
    let query = GroupActionsQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
        status,
        start_at_action_id,
        limit: count.map(|c| c as u16),
    };
    
    // Fetch actions with proof
    let (actions_result, metadata, proof) = GroupAction::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group actions with proof: {}", e)))?;
    
    // Convert result to response format
    let mut group_actions = Vec::new();
    for (action_id, action_opt) in actions_result {
        if let Some(_action) = action_opt {
            // For now, just return the action ID
            // The full action structure requires custom serialization
            group_actions.push(serde_json::json!({
                "actionId": action_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                // TODO: Serialize the full action event structure
            }));
        }
    }
    
    let data = serde_json::json!({
        "groupActions": group_actions
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_group_action_signers_with_proof_info(
    sdk: &WasmSdk,
    contract_id: &str,
    group_contract_position: u32,
    status: &str,
    action_id: &str,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse action ID
    let action_id = Identifier::from_string(
        action_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Parse status
    let status = match status {
        "ACTIVE" => GroupActionStatus::ActionActive,
        "CLOSED" => GroupActionStatus::ActionClosed,
        _ => return Err(JsError::new(&format!("Invalid status: {}. Must be ACTIVE or CLOSED", status))),
    };
    
    // Create query
    let query = GroupActionSignersQuery {
        contract_id,
        group_contract_position: group_contract_position as GroupContractPosition,
        status,
        action_id,
    };
    
    // Fetch signers with proof
    let (signers_result, metadata, proof) = GroupMemberPower::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch group action signers with proof: {}", e)))?;
    
    // Convert result to response format
    let mut signers = Vec::new();
    for (signer_id, power_opt) in signers_result {
        if let Some(power) = power_opt {
            signers.push(serde_json::json!({
                "signerId": signer_id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
                "power": power
            }));
        }
    }
    
    let data = serde_json::json!({
        "signers": signers
    });
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_groups_data_contracts_with_proof_info(
    sdk: &WasmSdk,
    data_contract_ids: Vec<String>,
) -> Result<JsValue, JsError> {
    let mut results: Vec<GroupsDataContractInfo> = Vec::new();
    let mut combined_metadata: Option<ResponseMetadata> = None;
    let mut combined_proof: Option<ProofInfo> = None;
    
    for contract_id_str in data_contract_ids {
        let contract_id = Identifier::from_string(
            &contract_id_str,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        
        // Fetch all groups for this contract with proof
        let query = GroupInfosQuery {
            contract_id,
            start_group_contract_position: None,
            limit: None,
        };
        
        let (groups_result, metadata, proof) = Group::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch groups for contract {} with proof: {}", contract_id_str, e)))?;
        
        // Store first metadata and proof
        if combined_metadata.is_none() {
            combined_metadata = Some(metadata.into());
            combined_proof = Some(proof.into());
        }
        
        let mut groups: Vec<GroupContractPositionInfo> = Vec::new();
        
        for (position, group_opt) in groups_result {
            if let Some(group) = group_opt {
                groups.push(GroupContractPositionInfo {
                    position: position as u32,
                    group: GroupInfoResponse::from_group(&group),
                });
            }
        }
        
        results.push(GroupsDataContractInfo {
            data_contract_id: contract_id_str,
            groups,
        });
    }
    
    let response = ProofMetadataResponse {
        data: results,
        metadata: combined_metadata.unwrap_or_else(|| ResponseMetadata {
            height: 0,
            core_chain_locked_height: 0,
            epoch: 0,
            time_ms: 0,
            protocol_version: 0,
            chain_id: String::new(),
        }),
        proof: combined_proof.unwrap_or_else(|| ProofInfo {
            grovedb_proof: String::new(),
            quorum_hash: String::new(),
            signature: String::new(),
            round: 0,
            block_id_hash: String::new(),
            quorum_type: 0,
        }),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}