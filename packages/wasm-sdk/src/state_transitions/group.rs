//! Group action state transitions
//!
//! This module provides WASM bindings for group-related state transitions.
//! Groups are used for collaborative actions like multi-sig operations, DAOs, etc.

use crate::error::to_js_error;
use dpp::data_contract::group::{Group, GroupMemberPower};
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::prelude::Identifier;
use dpp::serialization::{PlatformSerializable, PlatformDeserializable};
use dpp::state_transition::StateTransition;
use dpp::tokens::token_event::TokenEvent;
use js_sys::{Array, Object, Reflect, Uint8Array};
use platform_value::string_encoding::Encoding;
use wasm_bindgen::prelude::*;
use serde_json;

/// Group action types for JavaScript
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum GroupActionType {
    TokenTransfer = 0,
    TokenMint = 1,
    TokenBurn = 2,
    TokenFreeze = 3,
    TokenUnfreeze = 4,
    TokenSetPrice = 5,
    ContractUpdate = 6,
    GroupMemberAdd = 7,
    GroupMemberRemove = 8,
    GroupSettingsUpdate = 9,
    Custom = 10,
}

/// Create a group state transition info object
#[wasm_bindgen(js_name = createGroupStateTransitionInfo)]
pub fn create_group_state_transition_info(
    group_contract_position: u16,
    action_id: Option<String>,
    is_proposer: bool,
) -> Result<JsValue, JsError> {
    let info = if is_proposer {
        GroupStateTransitionInfo {
            group_contract_position,
            action_id: Identifier::default(),
            action_is_proposer: true,
        }
    } else {
        let action_id = action_id
            .ok_or_else(|| JsError::new("action_id is required when not proposer"))?;
        let id = Identifier::from_string(&action_id, Encoding::Base58)
            .map_err(|e| JsError::new(&format!("Invalid action ID: {}", e)))?;
        
        GroupStateTransitionInfo {
            group_contract_position,
            action_id: id,
            action_is_proposer: false,
        }
    };
    
    // Convert to JS object
    let obj = Object::new();
    Reflect::set(&obj, &"groupContractPosition".into(), &info.group_contract_position.into())
        .map_err(|_| JsError::new("Failed to set groupContractPosition"))?;
    Reflect::set(&obj, &"actionId".into(), &info.action_id.to_string(Encoding::Base58).into())
        .map_err(|_| JsError::new("Failed to set actionId"))?;
    Reflect::set(&obj, &"isProposer".into(), &info.action_is_proposer.into())
        .map_err(|_| JsError::new("Failed to set isProposer"))?;
    
    Ok(obj.into())
}

/// Parse group info from a JavaScript object
fn parse_group_info_from_js(js_obj: &JsValue) -> Result<GroupStateTransitionInfo, JsError> {
    let obj = js_obj.dyn_ref::<Object>()
        .ok_or_else(|| JsError::new("Expected a group info object"))?;
    
    let group_contract_position = Reflect::get(obj, &"groupContractPosition".into())
        .map_err(|_| JsError::new("Failed to get groupContractPosition"))?
        .as_f64()
        .ok_or_else(|| JsError::new("groupContractPosition must be a number"))? as u16;
    
    let is_proposer = Reflect::get(obj, &"isProposer".into())
        .map_err(|_| JsError::new("Failed to get isProposer"))?
        .as_bool()
        .unwrap_or_default();
    
    let info = if is_proposer {
        GroupStateTransitionInfo {
            group_contract_position,
            action_id: Identifier::default(),
            action_is_proposer: true,
        }
    } else {
        let action_id_str = Reflect::get(obj, &"actionId".into())
            .map_err(|_| JsError::new("Failed to get actionId"))?
            .as_string()
            .ok_or_else(|| JsError::new("actionId must be a string"))?;
        
        let action_id = Identifier::from_string(&action_id_str, Encoding::Base58)
            .map_err(|e| JsError::new(&format!("Invalid action ID: {}", e)))?;
        
        GroupStateTransitionInfo {
            group_contract_position,
            action_id,
            action_is_proposer: false,
        }
    };
    
    Ok(info)
}

/// Create a token event for group actions
#[wasm_bindgen(js_name = createTokenEventBytes)]
pub fn create_token_event_bytes(
    event_type: &str,
    token_position: u8,
    amount: Option<f64>,
    recipient_id: Option<String>,
    note: Option<String>,
) -> Result<Vec<u8>, JsError> {
    // This is a simplified version - in reality, TokenEvent has more complex structure
    // based on the event type. This would need to be expanded based on actual DPP implementation
    
    let mut event_bytes = Vec::new();
    
    // Event type byte
    let type_byte = match event_type {
        "transfer" => 0u8,
        "mint" => 1u8,
        "burn" => 2u8,
        "freeze" => 3u8,
        "unfreeze" => 4u8,
        _ => return Err(JsError::new(&format!("Unknown event type: {}", event_type))),
    };
    event_bytes.push(type_byte);
    
    // Token position
    event_bytes.push(token_position);
    
    // Amount (if applicable)
    if let Some(amt) = amount {
        event_bytes.push(1); // Has amount flag
        let amount_bytes = (amt * 1000.0) as u64; // Convert to smallest units
        event_bytes.extend_from_slice(&amount_bytes.to_le_bytes());
    } else {
        event_bytes.push(0); // No amount
    }
    
    // Recipient (if applicable)
    if let Some(recipient) = recipient_id {
        event_bytes.push(1); // Has recipient flag
        let id = Identifier::from_string(&recipient, Encoding::Base58)
            .map_err(|e| JsError::new(&format!("Invalid recipient ID: {}", e)))?;
        event_bytes.extend_from_slice(id.as_bytes());
    } else {
        event_bytes.push(0); // No recipient
    }
    
    // Note (if applicable)
    if let Some(note_text) = note {
        event_bytes.push(1); // Has note flag
        let note_bytes = note_text.as_bytes();
        event_bytes.extend_from_slice(&(note_bytes.len() as u16).to_le_bytes());
        event_bytes.extend_from_slice(note_bytes);
    } else {
        event_bytes.push(0); // No note
    }
    
    Ok(event_bytes)
}

/// Deserialize group action event from bytes
fn deserialize_group_action_event(event_bytes: &[u8]) -> Result<GroupActionEvent, JsError> {
    if event_bytes.is_empty() {
        return Err(JsError::new("Event bytes cannot be empty"));
    }
    
    let event_type = event_bytes[0];
    let mut pos = 1;
    
    match event_type {
        0 => { // Transfer
            // Parse token position
            if pos >= event_bytes.len() {
                return Err(JsError::new("Missing token position"));
            }
            let _token_position = event_bytes[pos];
            pos += 1;
            
            // Parse amount flag and amount
            if pos >= event_bytes.len() {
                return Err(JsError::new("Missing amount flag"));
            }
            let has_amount = event_bytes[pos] != 0;
            pos += 1;
            
            let amount = if has_amount {
                if pos + 8 > event_bytes.len() {
                    return Err(JsError::new("Insufficient bytes for amount"));
                }
                let amount_bytes: [u8; 8] = event_bytes[pos..pos+8].try_into()
                    .map_err(|_| JsError::new("Failed to parse amount bytes"))?;
                pos += 8;
                u64::from_le_bytes(amount_bytes)
            } else {
                return Err(JsError::new("Transfer event requires amount"));
            };
            
            // Parse recipient flag and recipient
            if pos >= event_bytes.len() {
                return Err(JsError::new("Missing recipient flag"));
            }
            let has_recipient = event_bytes[pos] != 0;
            pos += 1;
            
            let recipient_id = if has_recipient {
                if pos + 32 > event_bytes.len() {
                    return Err(JsError::new("Insufficient bytes for recipient ID"));
                }
                let id_bytes: [u8; 32] = event_bytes[pos..pos+32].try_into()
                    .map_err(|_| JsError::new("Failed to parse recipient ID"))?;
                pos += 32;
                Identifier::from_bytes(&id_bytes)
                    .map_err(|e| JsError::new(&format!("Invalid recipient ID: {}", e)))?
            } else {
                return Err(JsError::new("Transfer event requires recipient"));
            };
            
            // For now, create a basic transfer event
            // In production, this would parse additional fields like notes
            Ok(GroupActionEvent::TokenEvent(TokenEvent::Transfer(
                recipient_id,  // sender_identity_id (using recipient as placeholder)
                None,          // recipient_note
                None,          // sender_note_recipient_identity_id_amount
                None,          // recipient_note_recipient_identity_id_amount
                amount,
            )))
        },
        1 => { // Mint
            // Parse amount
            if pos + 8 > event_bytes.len() {
                return Err(JsError::new("Insufficient bytes for mint amount"));
            }
            let amount_bytes: [u8; 8] = event_bytes[pos..pos+8].try_into()
                .map_err(|_| JsError::new("Failed to parse amount bytes"))?;
            let amount = u64::from_le_bytes(amount_bytes);
            
            Ok(GroupActionEvent::TokenEvent(TokenEvent::Mint(
                amount,
                None, // note
            )))
        },
        2 => { // Burn
            // Parse amount
            if pos + 8 > event_bytes.len() {
                return Err(JsError::new("Insufficient bytes for burn amount"));
            }
            let amount_bytes: [u8; 8] = event_bytes[pos..pos+8].try_into()
                .map_err(|_| JsError::new("Failed to parse amount bytes"))?;
            let amount = u64::from_le_bytes(amount_bytes);
            
            Ok(GroupActionEvent::TokenEvent(TokenEvent::Burn(
                amount,
                None, // note
            )))
        },
        _ => Err(JsError::new(&format!("Unknown event type: {}", event_type))),
    }
}

/// Create a group action
#[wasm_bindgen(js_name = createGroupAction)]
pub fn create_group_action(
    contract_id: &str,
    proposer_id: &str,
    token_position: u16,
    event_bytes: &[u8],
) -> Result<Vec<u8>, JsError> {
    let contract_id = Identifier::from_string(contract_id, Encoding::Base58)
        .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    let proposer_id = Identifier::from_string(proposer_id, Encoding::Base58)
        .map_err(|e| JsError::new(&format!("Invalid proposer ID: {}", e)))?;
    
    // Deserialize event_bytes into GroupActionEvent
    let event = deserialize_group_action_event(event_bytes)?;
    
    let action = dpp::group::group_action::v0::GroupActionV0 {
        contract_id,
        proposer_id,
        token_contract_position: token_position,
        event,
    };
    
    let group_action = GroupAction::V0(action);
    
    group_action.serialize_to_bytes()
        .map_err(to_js_error)
}

/// Add group info to a state transition
#[wasm_bindgen(js_name = addGroupInfoToStateTransition)]
pub fn add_group_info_to_state_transition(
    state_transition_bytes: &[u8],
    group_info: JsValue,
) -> Result<Vec<u8>, JsError> {
    // Parse the state transition
    let mut state_transition = StateTransition::deserialize_from_bytes(state_transition_bytes)
        .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;
    
    // Parse group info
    let info = parse_group_info_from_js(&group_info)?;
    
    // Add group info to the state transition
    // Note: This is a simplified version. In reality, different state transition types
    // handle group info differently
    match &mut state_transition {
        StateTransition::DataContractUpdate(st) => {
            // DataContractUpdate supports group info
            // Note: The actual API to set group info on transitions may vary
            // This is a placeholder until the exact API is available
            return Err(JsError::new("Group info for DataContractUpdate requires platform support"));
        }
        StateTransition::Batch(st) => {
            // Batch transitions can have group info for certain document operations
            // Note: The actual API to set group info on transitions may vary
            // This is a placeholder until the exact API is available
            return Err(JsError::new("Group info for Batch transitions requires platform support"));
        }
        _ => {
            return Err(JsError::new("This state transition type does not support group info"));
        }
    }
}

/// Get group info from a state transition
#[wasm_bindgen(js_name = getGroupInfoFromStateTransition)]
pub fn get_group_info_from_state_transition(
    state_transition_bytes: &[u8],
) -> Result<JsValue, JsError> {
    let state_transition = StateTransition::deserialize_from_bytes(state_transition_bytes)
        .map_err(|e| JsError::new(&format!("Failed to deserialize state transition: {}", e)))?;
    
    // Extract group info based on transition type
    // Note: This is a simplified version
    match &state_transition {
        StateTransition::DataContractUpdate(_st) => {
            // TODO: Get group info from the transition when the API is available
            Ok(JsValue::null())
        }
        StateTransition::Batch(_st) => {
            // TODO: Get group info from the transition when the API is available
            Ok(JsValue::null())
        }
        _ => {
            Ok(JsValue::null())
        }
    }
}

/// Create a group member structure
#[wasm_bindgen(js_name = createGroupMember)]
pub fn create_group_member(
    identity_id: &str,
    power: u16,
) -> Result<JsValue, JsError> {
    let id = Identifier::from_string(identity_id, Encoding::Base58)
        .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let obj = Object::new();
    Reflect::set(&obj, &"identityId".into(), &identity_id.into())
        .map_err(|_| JsError::new("Failed to set identityId"))?;
    Reflect::set(&obj, &"power".into(), &power.into())
        .map_err(|_| JsError::new("Failed to set power"))?;
    
    Ok(obj.into())
}

/// Validate group configuration
#[wasm_bindgen(js_name = validateGroupConfig)]
pub fn validate_group_config(
    members: JsValue,
    required_power: u16,
    member_power_limit: Option<u16>,
) -> Result<JsValue, JsError> {
    let members_array = members.dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("members must be an array"))?;
    
    let mut total_power = 0u32;
    let mut member_count = 0;
    let power_limit = member_power_limit.unwrap_or(u16::MAX);
    
    for i in 0..members_array.length() {
        let member = members_array.get(i);
        let member_obj = member.dyn_ref::<Object>()
            .ok_or_else(|| JsError::new("Each member must be an object"))?;
        
        let power = Reflect::get(member_obj, &"power".into())
            .map_err(|_| JsError::new("Failed to get member power"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Member power must be a number"))? as u16;
        
        if power == 0 {
            return Err(JsError::new("Member power cannot be zero"));
        }
        
        if power > power_limit {
            return Err(JsError::new(&format!(
                "Member power {} exceeds limit {}",
                power, power_limit
            )));
        }
        
        total_power += power as u32;
        member_count += 1;
    }
    
    if member_count == 0 {
        return Err(JsError::new("Group must have at least one member"));
    }
    
    if total_power < required_power as u32 {
        return Err(JsError::new(&format!(
            "Total power {} is less than required power {}",
            total_power, required_power
        )));
    }
    
    // Return validation result
    let result = Object::new();
    Reflect::set(&result, &"valid".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set valid"))?;
    Reflect::set(&result, &"totalPower".into(), &total_power.into())
        .map_err(|_| JsError::new("Failed to set totalPower"))?;
    Reflect::set(&result, &"memberCount".into(), &member_count.into())
        .map_err(|_| JsError::new("Failed to set memberCount"))?;
    Reflect::set(&result, &"hasRequiredPower".into(), &(total_power >= required_power as u32).into())
        .map_err(|_| JsError::new("Failed to set hasRequiredPower"))?;
    
    Ok(result.into())
}

/// Calculate if a group action has enough approvals
#[wasm_bindgen(js_name = calculateGroupActionApproval)]
pub fn calculate_group_action_approval(
    approvals: JsValue,
    required_power: u16,
) -> Result<JsValue, JsError> {
    let approvals_array = approvals.dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("approvals must be an array"))?;
    
    let mut total_approval_power = 0u32;
    let mut approval_count = 0;
    
    for i in 0..approvals_array.length() {
        let approval = approvals_array.get(i);
        let approval_obj = approval.dyn_ref::<Object>()
            .ok_or_else(|| JsError::new("Each approval must be an object"))?;
        
        let power = Reflect::get(approval_obj, &"power".into())
            .map_err(|_| JsError::new("Failed to get approval power"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Approval power must be a number"))? as u16;
        
        total_approval_power += power as u32;
        approval_count += 1;
    }
    
    let is_approved = total_approval_power >= required_power as u32;
    
    // Return result
    let result = Object::new();
    Reflect::set(&result, &"approved".into(), &is_approved.into())
        .map_err(|_| JsError::new("Failed to set approved"))?;
    Reflect::set(&result, &"totalApprovalPower".into(), &total_approval_power.into())
        .map_err(|_| JsError::new("Failed to set totalApprovalPower"))?;
    Reflect::set(&result, &"requiredPower".into(), &required_power.into())
        .map_err(|_| JsError::new("Failed to set requiredPower"))?;
    Reflect::set(&result, &"approvalCount".into(), &approval_count.into())
        .map_err(|_| JsError::new("Failed to set approvalCount"))?;
    Reflect::set(&result, &"remainingPower".into(), 
        &(if is_approved { 0 } else { (required_power as u32) - total_approval_power }).into())
        .map_err(|_| JsError::new("Failed to set remainingPower"))?;
    
    Ok(result.into())
}

/// Helper to create a group configuration for data contracts
#[wasm_bindgen(js_name = createGroupConfiguration)]
pub fn create_group_configuration(
    position: u8,
    required_power: u16,
    member_power_limit: Option<u16>,
    members: JsValue,
) -> Result<JsValue, JsError> {
    // Validate the configuration first
    validate_group_config(members.clone(), required_power, member_power_limit)?;
    
    let config = Object::new();
    Reflect::set(&config, &"position".into(), &position.into())
        .map_err(|_| JsError::new("Failed to set position"))?;
    Reflect::set(&config, &"requiredPower".into(), &required_power.into())
        .map_err(|_| JsError::new("Failed to set requiredPower"))?;
    
    if let Some(limit) = member_power_limit {
        Reflect::set(&config, &"memberPowerLimit".into(), &limit.into())
            .map_err(|_| JsError::new("Failed to set memberPowerLimit"))?;
    }
    
    Reflect::set(&config, &"members".into(), &members)
        .map_err(|_| JsError::new("Failed to set members"))?;
    
    Ok(config.into())
}

/// Deserialize a group event from bytes
#[wasm_bindgen(js_name = deserializeGroupEvent)]
pub fn deserialize_group_event(event_bytes: &[u8]) -> Result<JsValue, JsError> {
    let event = deserialize_group_action_event(event_bytes)?;
    
    // Convert to JavaScript object
    let obj = Object::new();
    
    match event {
        GroupActionEvent::TokenEvent(token_event) => {
            Reflect::set(&obj, &"type".into(), &"token".into())
                .map_err(|_| JsError::new("Failed to set event type"))?;
            
            match token_event {
                TokenEvent::Transfer(sender_id, recipient_note, sender_note, recipient_note2, amount) => {
                    Reflect::set(&obj, &"eventType".into(), &"transfer".into())
                        .map_err(|_| JsError::new("Failed to set event type"))?;
                    Reflect::set(&obj, &"senderId".into(), &sender_id.to_string(Encoding::Base58).into())
                        .map_err(|_| JsError::new("Failed to set sender ID"))?;
                    Reflect::set(&obj, &"amount".into(), &(amount as f64).into())
                        .map_err(|_| JsError::new("Failed to set amount"))?;
                },
                TokenEvent::Mint(amount, note) => {
                    Reflect::set(&obj, &"eventType".into(), &"mint".into())
                        .map_err(|_| JsError::new("Failed to set event type"))?;
                    Reflect::set(&obj, &"amount".into(), &(amount as f64).into())
                        .map_err(|_| JsError::new("Failed to set amount"))?;
                },
                TokenEvent::Burn(amount, note) => {
                    Reflect::set(&obj, &"eventType".into(), &"burn".into())
                        .map_err(|_| JsError::new("Failed to set event type"))?;
                    Reflect::set(&obj, &"amount".into(), &(amount as f64).into())
                        .map_err(|_| JsError::new("Failed to set amount"))?;
                },
                _ => {
                    Reflect::set(&obj, &"eventType".into(), &"unknown".into())
                        .map_err(|_| JsError::new("Failed to set event type"))?;
                }
            }
        },
        _ => {
            Reflect::set(&obj, &"type".into(), &"unknown".into())
                .map_err(|_| JsError::new("Failed to set event type"))?;
        }
    }
    
    Ok(obj.into())
}

/// Serialize a group event from JavaScript object
#[wasm_bindgen(js_name = serializeGroupEvent)]  
pub fn serialize_group_event(event_obj: JsValue) -> Result<Vec<u8>, JsError> {
    let obj = event_obj.dyn_ref::<Object>()
        .ok_or_else(|| JsError::new("Event must be an object"))?;
    
    let event_type = Reflect::get(obj, &"eventType".into())
        .map_err(|_| JsError::new("Failed to get eventType"))?
        .as_string()
        .ok_or_else(|| JsError::new("eventType must be a string"))?;
    
    match event_type.as_str() {
        "transfer" => {
            let token_position = Reflect::get(obj, &"tokenPosition".into())
                .map_err(|_| JsError::new("Failed to get tokenPosition"))?
                .as_f64()
                .ok_or_else(|| JsError::new("tokenPosition must be a number"))? as u8;
                
            let amount = Reflect::get(obj, &"amount".into())
                .map_err(|_| JsError::new("Failed to get amount"))?
                .as_f64()
                .ok_or_else(|| JsError::new("amount must be a number"))?;
                
            let recipient_id = Reflect::get(obj, &"recipientId".into())
                .map_err(|_| JsError::new("Failed to get recipientId"))?
                .as_string()
                .ok_or_else(|| JsError::new("recipientId must be a string"))?;
            
            create_token_event_bytes("transfer", token_position, Some(amount), Some(recipient_id), None)
        },
        "mint" => {
            let token_position = Reflect::get(obj, &"tokenPosition".into())
                .map_err(|_| JsError::new("Failed to get tokenPosition"))?
                .as_f64()
                .ok_or_else(|| JsError::new("tokenPosition must be a number"))? as u8;
                
            let amount = Reflect::get(obj, &"amount".into())
                .map_err(|_| JsError::new("Failed to get amount"))?
                .as_f64()
                .ok_or_else(|| JsError::new("amount must be a number"))?;
            
            create_token_event_bytes("mint", token_position, Some(amount), None, None)
        },
        "burn" => {
            let token_position = Reflect::get(obj, &"tokenPosition".into())
                .map_err(|_| JsError::new("Failed to get tokenPosition"))?
                .as_f64()
                .ok_or_else(|| JsError::new("tokenPosition must be a number"))? as u8;
                
            let amount = Reflect::get(obj, &"amount".into())
                .map_err(|_| JsError::new("Failed to get amount"))?
                .as_f64()
                .ok_or_else(|| JsError::new("amount must be a number"))?;
            
            create_token_event_bytes("burn", token_position, Some(amount), None, None)
        },
        _ => Err(JsError::new(&format!("Unknown event type: {}", event_type))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_group_state_transition_info() {
        // Test proposer info
        let info = create_group_state_transition_info(1, None, true).unwrap();
        assert!(!info.is_null());
        
        // Test non-proposer info
        let action_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq";
        let info = create_group_state_transition_info(2, Some(action_id.to_string()), false).unwrap();
        assert!(!info.is_null());
    }
}