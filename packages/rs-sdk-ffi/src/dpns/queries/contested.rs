//! FFI bindings for contested DPNS username queries

use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::DocumentV0Getters;
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKContender, DashSDKContestInfo, DashSDKContestedName, DashSDKContestedNamesList,
    DashSDKNameTimestamp, DashSDKNameTimestampList, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::identifier::Identifier;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use serde_json::json; // Still used by other functions

/// Get all contested DPNS usernames where an identity is a contender
///
/// # Safety
/// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
/// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
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
    let identity = match Identifier::from_string(identity_id_str, Encoding::Base58) {
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
        sdk.get_contested_dpns_usernames_by_identity(identity, limit_opt)
            .await
    });

    match result {
        Ok(contested_names) => {
            // Convert results to JSON array
            let mut usernames = Vec::new();
            for contested_name in contested_names {
                let mut name_map = serde_json::Map::new();
                name_map.insert("label".to_string(), json!(contested_name.label));
                name_map.insert(
                    "normalizedLabel".to_string(),
                    json!(contested_name.normalized_label),
                );

                // Convert contenders to array of base58 strings
                let contenders: Vec<String> = contested_name
                    .contenders
                    .into_iter()
                    .map(|id| id.to_string(Encoding::Base58))
                    .collect();
                name_map.insert("contenders".to_string(), json!(contenders));

                usernames.push(json!(name_map));
            }

            match serde_json::to_string(&usernames) {
                Ok(json_str) => match CString::new(json_str) {
                    Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                    Err(_) => DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        "Failed to create C string".to_string(),
                    )),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                )),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        )),
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
        sdk.get_contested_dpns_vote_state(label_str, limit_opt)
            .await
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
                        result_map
                            .insert("winner".to_string(), json!(id.to_string(Encoding::Base58)));
                    }
                    ContestedDocumentVotePollWinnerInfo::Locked => {
                        result_map.insert("winner".to_string(), json!("LOCKED"));
                    }
                    ContestedDocumentVotePollWinnerInfo::NoWinner => {
                        result_map.insert("winner".to_string(), json!(null));
                    }
                }
            }

            // Add contenders
            let mut contenders_array = Vec::new();
            for (contender_id, votes) in contenders.contenders {
                let mut contender_map = serde_json::Map::new();
                contender_map.insert(
                    "identifier".to_string(),
                    json!(contender_id.to_string(Encoding::Base58)),
                );
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
                Ok(json_str) => match CString::new(json_str) {
                    Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                    Err(_) => DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        "Failed to create C string".to_string(),
                    )),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                )),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        )),
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
        sdk.get_contested_dpns_normalized_usernames(limit_opt, start_after_opt)
            .await
    });

    match result {
        Ok(contested_names) => {
            // The result is now a simple Vec<String> of normalized usernames
            // Just convert directly to JSON array of strings
            match serde_json::to_string(&contested_names) {
                Ok(json_str) => match CString::new(json_str) {
                    Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                    Err(_) => DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        "Failed to create C string".to_string(),
                    )),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                )),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        )),
    }
}

/// Get current DPNS contests (active vote polls)
///
/// Returns a list of contested DPNS names with their end times.
/// The caller is responsible for freeing the result with `dash_sdk_name_timestamp_list_free`.
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_current_contests(
    sdk_handle: *const SDKHandle,
    start_time: u64, // 0 means no start time filter
    end_time: u64,   // 0 means no end time filter
    limit: u16,      // 0 means use default limit (100)
) -> *mut DashSDKNameTimestampList {
    if sdk_handle.is_null() {
        return std::ptr::null_mut();
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    let start_time_opt = if start_time > 0 {
        Some(start_time)
    } else {
        None
    };
    let end_time_opt = if end_time > 0 { Some(end_time) } else { None };
    let limit_opt = if limit > 0 { Some(limit) } else { None };

    let result = sdk_wrapper.runtime.block_on(async {
        sdk.get_current_dpns_contests(start_time_opt, end_time_opt, limit_opt)
            .await
    });

    match result {
        Ok(contests) => {
            let count = contests.len();
            let mut entries = Vec::with_capacity(count);

            for (name, end_time) in contests {
                let c_name = match CString::new(name) {
                    Ok(s) => s.into_raw(),
                    Err(_) => continue,
                };

                entries.push(DashSDKNameTimestamp {
                    name: c_name,
                    end_time,
                });
            }

            let count = entries.len();
            let entries_ptr = if entries.is_empty() {
                std::ptr::null_mut()
            } else {
                let boxed = entries.into_boxed_slice();
                Box::into_raw(boxed) as *mut DashSDKNameTimestamp
            };

            let list = Box::new(DashSDKNameTimestampList {
                entries: entries_ptr,
                count,
            });

            Box::into_raw(list)
        }
        Err(_) => std::ptr::null_mut(),
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
    let identity = match Identifier::from_string(identity_id_str, Encoding::Base58) {
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
        sdk.get_contested_dpns_identity_votes(identity, limit_opt, offset_opt)
            .await
    });

    match result {
        Ok(contested_names) => {
            // Convert results to JSON array
            let mut usernames = Vec::new();
            for contested_name in contested_names {
                let mut name_map = serde_json::Map::new();
                name_map.insert("label".to_string(), json!(contested_name.label));
                name_map.insert(
                    "normalizedLabel".to_string(),
                    json!(contested_name.normalized_label),
                );

                // Convert contenders to array of base58 strings
                let contenders: Vec<String> = contested_name
                    .contenders
                    .into_iter()
                    .map(|id| id.to_string(Encoding::Base58))
                    .collect();
                name_map.insert("contenders".to_string(), json!(contenders));

                usernames.push(json!(name_map));
            }

            match serde_json::to_string(&usernames) {
                Ok(json_str) => match CString::new(json_str) {
                    Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                    Err(_) => DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        "Failed to create C string".to_string(),
                    )),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("JSON serialization error: {}", e),
                )),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("SDK error: {}", e),
        )),
    }
}

/// Get non-resolved DPNS contests for a specific identity
///
/// Returns a list of contested but unresolved DPNS usernames where the identity is a contender.
/// The caller is responsible for freeing the result with `dash_sdk_contested_names_list_free`.
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_non_resolved_contests_for_identity(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    limit: u32,
) -> *mut DashSDKContestedNamesList {
    if sdk_handle.is_null() || identity_id.is_null() {
        return std::ptr::null_mut();
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    let identity_id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Parse identity ID
    let identity = match Identifier::from_string(identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(_) => return std::ptr::null_mut(),
    };

    let limit_opt = if limit > 0 { Some(limit) } else { None };

    let result = sdk_wrapper.runtime.block_on(async {
        sdk.get_non_resolved_dpns_contests_for_identity(identity, limit_opt)
            .await
    });

    match result {
        Ok(names_with_contest_info) => {
            // Baked-in system contract: no network round trip, and it is the
            // schema these documents were written against.
            let platform_version = sdk.version();
            let dpns_domain_type = load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .ok()
                .and_then(|contract| contract.document_type_cloned_for_name("domain").ok());

            let count = names_with_contest_info.len();
            let mut names = Vec::with_capacity(count);

            for (name, contest_info) in names_with_contest_info {
                // Convert name to C string
                let c_name = match CString::new(name) {
                    Ok(s) => s.into_raw(),
                    Err(_) => continue,
                };

                // Convert contenders
                let contender_count = contest_info.contenders.contenders.len();
                let mut contenders = Vec::with_capacity(contender_count);

                for (contender_id, votes) in contest_info.contenders.contenders {
                    let id_str = contender_id.to_string(Encoding::Base58);
                    let c_id = match CString::new(id_str) {
                        Ok(s) => s.into_raw(),
                        Err(_) => continue,
                    };

                    // Extract actual vote tally from ContenderWithSerializedDocument
                    let vote_count = votes.vote_tally().unwrap_or(0);

                    // The requester's own spelling ("pizza"), which the
                    // normalized contest name ("p1zza") does not preserve.
                    // Null when the document can't be decoded — callers fall
                    // back to the normalized name rather than guessing.
                    let c_label = dpns_domain_type
                        .as_ref()
                        .and_then(|doc_type| {
                            let decoded = votes
                                .try_to_contender(doc_type.as_ref(), platform_version)
                                .ok()?;
                            let label = decoded.document().as_ref()?.get("label")?.as_str()?;
                            CString::new(label).ok().map(|s| s.into_raw())
                        })
                        .unwrap_or(std::ptr::null_mut());

                    contenders.push(DashSDKContender {
                        identity_id: c_id,
                        vote_count,
                        label: c_label,
                    });
                }

                let contender_count = contenders.len();
                let contenders_ptr = if contenders.is_empty() {
                    std::ptr::null_mut()
                } else {
                    let boxed = contenders.into_boxed_slice();
                    Box::into_raw(boxed) as *mut DashSDKContender
                };

                let contest_info_c = DashSDKContestInfo {
                    contenders: contenders_ptr,
                    contender_count,
                    abstain_votes: contest_info.contenders.abstain_vote_tally.unwrap_or(0),
                    lock_votes: contest_info.contenders.lock_vote_tally.unwrap_or(0),
                    end_time: contest_info.end_time,
                    has_winner: contest_info.contenders.winner.is_some(),
                };

                names.push(DashSDKContestedName {
                    name: c_name,
                    contest_info: contest_info_c,
                });
            }

            let names_count = names.len();
            let names_ptr = if names.is_empty() {
                std::ptr::null_mut()
            } else {
                let boxed = names.into_boxed_slice();
                Box::into_raw(boxed) as *mut DashSDKContestedName
            };

            let list = Box::new(DashSDKContestedNamesList {
                names: names_ptr,
                count: names_count,
            });

            Box::into_raw(list)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get contested DPNS usernames that are not yet resolved
///
/// Returns a list of contested but unresolved DPNS usernames with their contest information.
/// The caller is responsible for freeing the result with `dash_sdk_contested_names_list_free`.
///
/// # Safety
/// This function is unsafe because it operates on raw pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_contested_non_resolved_usernames(
    sdk_handle: *const SDKHandle,
    limit: u32,
) -> *mut DashSDKContestedNamesList {
    if sdk_handle.is_null() {
        return std::ptr::null_mut();
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    let limit_opt = if limit > 0 { Some(limit) } else { None };

    let result = sdk_wrapper
        .runtime
        .block_on(async { sdk.get_contested_non_resolved_usernames(limit_opt).await });

    match result {
        Ok(names_with_contest_info) => {
            // Baked-in system contract: no network round trip, and it is the
            // schema these documents were written against.
            let platform_version = sdk.version();
            let dpns_domain_type = load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .ok()
                .and_then(|contract| contract.document_type_cloned_for_name("domain").ok());

            let count = names_with_contest_info.len();
            let mut names = Vec::with_capacity(count);

            for (name, contest_info) in names_with_contest_info {
                // Convert name to C string
                let c_name = match CString::new(name) {
                    Ok(s) => s.into_raw(),
                    Err(_) => continue,
                };

                // Convert contenders
                let contender_count = contest_info.contenders.contenders.len();
                let mut contenders = Vec::with_capacity(contender_count);

                for (contender_id, votes) in contest_info.contenders.contenders {
                    let id_str = contender_id.to_string(Encoding::Base58);
                    let c_id = match CString::new(id_str) {
                        Ok(s) => s.into_raw(),
                        Err(_) => continue,
                    };

                    // Extract actual vote tally from ContenderWithSerializedDocument
                    let vote_count = votes.vote_tally().unwrap_or(0);

                    // The requester's own spelling ("pizza"), which the
                    // normalized contest name ("p1zza") does not preserve.
                    // Null when the document can't be decoded — callers fall
                    // back to the normalized name rather than guessing.
                    let c_label = dpns_domain_type
                        .as_ref()
                        .and_then(|doc_type| {
                            let decoded = votes
                                .try_to_contender(doc_type.as_ref(), platform_version)
                                .ok()?;
                            let label = decoded.document().as_ref()?.get("label")?.as_str()?;
                            CString::new(label).ok().map(|s| s.into_raw())
                        })
                        .unwrap_or(std::ptr::null_mut());

                    contenders.push(DashSDKContender {
                        identity_id: c_id,
                        vote_count,
                        label: c_label,
                    });
                }

                let contender_count = contenders.len();
                let contenders_ptr = if contenders.is_empty() {
                    std::ptr::null_mut()
                } else {
                    let boxed = contenders.into_boxed_slice();
                    Box::into_raw(boxed) as *mut DashSDKContender
                };

                let contest_info_c = DashSDKContestInfo {
                    contenders: contenders_ptr,
                    contender_count,
                    abstain_votes: contest_info.contenders.abstain_vote_tally.unwrap_or(0),
                    lock_votes: contest_info.contenders.lock_vote_tally.unwrap_or(0),
                    end_time: contest_info.end_time,
                    has_winner: contest_info.contenders.winner.is_some(),
                };

                names.push(DashSDKContestedName {
                    name: c_name,
                    contest_info: contest_info_c,
                });
            }

            let names_count = names.len();
            let names_ptr = if names.is_empty() {
                std::ptr::null_mut()
            } else {
                let boxed = names.into_boxed_slice();
                Box::into_raw(boxed) as *mut DashSDKContestedName
            };

            let list = Box::new(DashSDKContestedNamesList {
                names: names_ptr,
                count: names_count,
            });

            Box::into_raw(list)
        }
        Err(_) => std::ptr::null_mut(),
    }
}
