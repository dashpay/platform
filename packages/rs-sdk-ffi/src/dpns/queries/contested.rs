//! FFI bindings for contested DPNS username queries

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::identifier::Identifier;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use serde_json::json;

/// Get all contested DPNS usernames where an identity is a contender
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_contested_usernames_by_identity(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    limit: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;
    
    let identity_id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };
    
    // Parse identity ID
    let identity = match Identifier::from_string(
        identity_id_str,
        Encoding::Base58,
    ) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };
    
    let limit_opt = if limit > 0 { Some(limit) } else { None };
    
    let result = sdk_wrapper.runtime.block_on(async {
        sdk.get_contested_dpns_usernames_by_identity(identity, limit_opt).await
    });
    
    match result {
        Ok(contested_names) => {
            // Convert results to JSON array
            let mut usernames = Vec::new();
            for contested_name in contested_names {
                let mut name_map = serde_json::Map::new();
                name_map.insert("label".to_string(), json!(contested_name.label));
                name_map.insert("normalizedLabel".to_string(), json!(contested_name.normalized_label));
                
                // Convert contenders to array of base58 strings
                let contenders: Vec<String> = contested_name.contenders.into_iter()
                    .map(|id| id.to_string(Encoding::Base58))
                    .collect();
                name_map.insert("contenders".to_string(), json!(contenders));
                
                usernames.push(json!(name_map));
            }
            
            match serde_json::to_string(&usernames) {
                Ok(json_str) => {
                    match CString::new(json_str) {
                        Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                        Err(_) => DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            "Failed to create C string".to_string(),
                        )),
                    }
                }
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                ))
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        ))
    }
}

/// Get the vote state for a contested DPNS username
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_contested_vote_state(
    sdk_handle: *const SDKHandle,
    label: *const c_char,
    limit: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if label.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Label is null".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;
    
    let label_str = match CStr::from_ptr(label).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };
    
    let limit_opt = if limit > 0 { Some(limit) } else { None };
    
    let result = sdk_wrapper.runtime.block_on(async {
        sdk.get_contested_dpns_vote_state(label_str, limit_opt).await
    });
    
    match result {
        Ok(contenders) => {
            // Convert Contenders to JSON
            let mut result_map = serde_json::Map::new();
            
            // Add winner if present
            if let Some((winner_info, _block_info)) = contenders.winner {
                use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
                match winner_info {
                    ContestedDocumentVotePollWinnerInfo::WonByIdentity(id) => {
                        result_map.insert("winner".to_string(), json!(id.to_string(Encoding::Base58)));
                    },
                    ContestedDocumentVotePollWinnerInfo::Locked => {
                        result_map.insert("winner".to_string(), json!("LOCKED"));
                    },
                    ContestedDocumentVotePollWinnerInfo::NoWinner => {
                        result_map.insert("winner".to_string(), json!(null));
                    }
                }
            }
            
            // Add contenders
            let mut contenders_array = Vec::new();
            for (contender_id, votes) in contenders.contenders {
                let mut contender_map = serde_json::Map::new();
                contender_map.insert("identifier".to_string(), json!(
                    contender_id.to_string(Encoding::Base58)
                ));
                // Convert votes to a simple format
                contender_map.insert("votes".to_string(), json!(format!("{:?}", votes)));
                contenders_array.push(json!(contender_map));
            }
            result_map.insert("contenders".to_string(), json!(contenders_array));
            
            // Add vote tallies if present
            if let Some(abstain_votes) = contenders.abstain_vote_tally {
                result_map.insert("abstainVotes".to_string(), json!(abstain_votes));
            }
            if let Some(lock_votes) = contenders.lock_vote_tally {
                result_map.insert("lockVotes".to_string(), json!(lock_votes));
            }
            
            match serde_json::to_string(&result_map) {
                Ok(json_str) => {
                    match CString::new(json_str) {
                        Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                        Err(_) => DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            "Failed to create C string".to_string(),
                        )),
                    }
                }
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                ))
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        ))
    }
}

/// Get all contested DPNS usernames
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_all_contested_usernames(
    sdk_handle: *const SDKHandle,
    limit: u32,
    start_after: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;
    
    let start_after_opt = if start_after.is_null() {
        None
    } else {
        match CStr::from_ptr(start_after).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                return DashSDKResult::error(FFIError::from(e).into());
            }
        }
    };
    
    let limit_opt = if limit > 0 { Some(limit) } else { None };
    
    let result = sdk_wrapper.runtime.block_on(async {
        sdk.get_contested_dpns_normalized_usernames(limit_opt, start_after_opt).await
    });
    
    match result {
        Ok(contested_names) => {
            // The result is now a simple Vec<String> of normalized usernames
            // Just convert directly to JSON array of strings
            match serde_json::to_string(&contested_names) {
                Ok(json_str) => {
                    match CString::new(json_str) {
                        Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                        Err(_) => DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            "Failed to create C string".to_string(),
                        )),
                    }
                }
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                ))
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        ))
    }
}

/// Get all contested DPNS usernames that an identity has voted on
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_identity_votes(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    limit: u32,
    offset: u16,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;
    
    let identity_id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };
    
    // Parse identity ID
    let identity = match Identifier::from_string(
        identity_id_str,
        Encoding::Base58,
    ) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };
    
    let limit_opt = if limit > 0 { Some(limit) } else { None };
    let offset_opt = if offset > 0 { Some(offset) } else { None };
    
    let result = sdk_wrapper.runtime.block_on(async {
        sdk.get_contested_dpns_identity_votes(identity, limit_opt, offset_opt).await
    });
    
    match result {
        Ok(contested_names) => {
            // Convert results to JSON array
            let mut usernames = Vec::new();
            for contested_name in contested_names {
                let mut name_map = serde_json::Map::new();
                name_map.insert("label".to_string(), json!(contested_name.label));
                name_map.insert("normalizedLabel".to_string(), json!(contested_name.normalized_label));
                
                // Convert contenders to array of base58 strings
                let contenders: Vec<String> = contested_name.contenders.into_iter()
                    .map(|id| id.to_string(Encoding::Base58))
                    .collect();
                name_map.insert("contenders".to_string(), json!(contenders));
                
                usernames.push(json!(name_map));
            }
            
            match serde_json::to_string(&usernames) {
                Ok(json_str) => {
                    match CString::new(json_str) {
                        Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                        Err(_) => DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            "Failed to create C string".to_string(),
                        )),
                    }
                }
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                ))
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        ))
    }
}