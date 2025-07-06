use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::{Fetch, FetchMany, Identifier};
use dash_sdk::dpp::data_contract::group::Group;
use dash_sdk::dpp::data_contract::GroupContractPosition;
use dash_sdk::dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dash_sdk::platform::group_actions::{GroupQuery, GroupInfosQuery};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GroupInfoResponse {
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
struct GroupsDataContractInfo {
    data_contract_id: String,
    groups: Vec<GroupContractPositionInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GroupContractPositionInfo {
    position: u32,
    group: GroupInfoResponse,
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