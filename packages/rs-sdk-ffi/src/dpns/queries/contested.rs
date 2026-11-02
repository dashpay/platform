//! FFI bindings for contested DPNS username queries

use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::Duration;

use crate::contested_resource::contender_document::ffi_contender;
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKContender, DashSDKContestInfo, DashSDKContestedName, DashSDKContestedNamesList,
    DashSDKNameTimestamp, DashSDKNameTimestampList, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::identifier::Identifier;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dash_sdk::platform::dpns_usernames::ContestRequests;
use dash_sdk::query_types::Contenders;
use serde_json::json; // Still used by other functions

/// A list query fans out to the vote state of every current contest. Keep one
/// slow or unreachable DAPI node from pinning an FFI caller indefinitely.
const DPNS_ACTIVE_CONTESTS_TIMEOUT: Duration = Duration::from_secs(30);

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
            let dpns_domain_type =
                load_system_data_contract(SystemDataContract::DPNS, platform_version)
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

                let contest_info_c = contest_info_to_ffi(
                    contest_info.contenders,
                    contest_info.end_time,
                    dpns_domain_type.as_ref().map(|doc_type| doc_type.as_ref()),
                    platform_version,
                );

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

    let result = sdk_wrapper.runtime.block_on(async {
        tokio::time::timeout(
            DPNS_ACTIVE_CONTESTS_TIMEOUT,
            sdk.get_contested_non_resolved_usernames(limit_opt),
        )
        .await
        .map_err(|_| {
            dash_sdk::Error::TimeoutReached(
                DPNS_ACTIVE_CONTESTS_TIMEOUT,
                "fetching active DPNS contests".to_string(),
            )
        })?
    });

    match result {
        Ok(names_with_contest_info) => {
            // Baked-in system contract: no network round trip, and it is the
            // schema these documents were written against.
            let platform_version = sdk.version();
            let dpns_domain_type =
                load_system_data_contract(SystemDataContract::DPNS, platform_version)
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

                let contest_info_c = contest_info_to_ffi(
                    contest_info.contenders,
                    contest_info.end_time,
                    dpns_domain_type.as_ref().map(|doc_type| doc_type.as_ref()),
                    platform_version,
                );

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

/// One contest in its C shape, shared by both list queries.
///
/// The SDK decodes the contenders' requests and derives the contest's
/// requested labels (`ContestRequests`); this only allocates their C shapes.
fn contest_info_to_ffi(
    contenders: Contenders,
    end_time: u64,
    dpns_domain_type: Option<DocumentTypeRef>,
    platform_version: &PlatformVersion,
) -> DashSDKContestInfo {
    // A contender whose id cannot be represented as a C string is left out
    // before its request is decoded, so it contributes no label either.
    let kept: Vec<(CString, ContenderWithSerializedDocument)> = contenders
        .contenders
        .into_iter()
        .filter_map(|(contender_id, votes)| {
            let c_id = CString::new(contender_id.to_string(Encoding::Base58)).ok()?;
            Some((c_id, votes))
        })
        .collect();
    let contest = ContestRequests::decode(
        kept.iter().map(|(_, votes)| votes),
        dpns_domain_type,
        platform_version,
    );

    let requested_labels: Vec<*mut c_char> = contest
        .requested_labels
        .into_iter()
        .filter_map(|label| CString::new(label).ok())
        .map(CString::into_raw)
        .collect();

    // `ContestRequests::decode` keeps one request per contender, in order.
    let ffi_contenders: Vec<DashSDKContender> = kept
        .into_iter()
        .zip(contest.requests)
        .map(|((c_id, votes), request)| {
            ffi_contender(request, c_id.into_raw(), votes.vote_tally().unwrap_or(0))
        })
        .collect();

    let requested_label_count = requested_labels.len();
    let requested_labels_ptr = if requested_labels.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(requested_labels.into_boxed_slice()) as *mut *mut c_char
    };

    let contender_count = ffi_contenders.len();
    let contenders_ptr = if ffi_contenders.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(ffi_contenders.into_boxed_slice()) as *mut DashSDKContender
    };

    DashSDKContestInfo {
        contenders: contenders_ptr,
        contender_count,
        requested_labels: requested_labels_ptr,
        requested_label_count,
        abstain_votes: contenders.abstain_vote_tally.unwrap_or(0),
        lock_votes: contenders.lock_vote_tally.unwrap_or(0),
        end_time,
        has_winner: contenders.winner.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contested_resource::contender_document::test_support::{
        requester_document, serialized_contender, CREATED_AT_MS,
    };
    use crate::types::dash_sdk_contest_info_free;
    use dash_sdk::dpp::document::DocumentV0Getters;
    use dash_sdk::dpp::voting::contender_structs::{
        ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
    };
    use std::collections::BTreeMap;

    /// A contest whose requests are `(owner, label, $createdAt)`; a `None`
    /// label stands for a contender whose document does not decode.
    fn contest(requests: &[(u8, Option<&str>, u64)]) -> DashSDKContestInfo {
        let contenders = requests
            .iter()
            .map(|(owner, label, created_at)| match label {
                Some(label) => {
                    let document = requester_document(*owner, label, Some(*created_at));
                    (document.owner_id(), serialized_contender(document))
                }
                None => {
                    let identity_id = Identifier::from([*owner; 32]);
                    let contender =
                        ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                            identity_id,
                            serialized_document: Some(vec![0xde, 0xad]),
                            vote_tally: Some(1),
                        });
                    (identity_id, contender)
                }
            })
            .collect::<BTreeMap<_, _>>();
        let platform_version = PlatformVersion::latest();
        let dpns_domain_type =
            load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .expect("DPNS system contract")
                .document_type_cloned_for_name("domain")
                .expect("domain");
        contest_info_to_ffi(
            Contenders {
                winner: None,
                contenders,
                abstain_vote_tally: Some(2),
                lock_vote_tally: Some(5),
            },
            42,
            Some(dpns_domain_type.as_ref()),
            platform_version,
        )
    }

    unsafe fn c_str(ptr: *mut c_char) -> String {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }

    #[test]
    fn should_carry_each_contenders_label_and_date() {
        let info = contest(&[(1, Some("pizza"), CREATED_AT_MS), (2, None, 0)]);
        unsafe {
            assert_eq!(info.contender_count, 2);
            let contenders = std::slice::from_raw_parts(info.contenders, 2);
            let first = &contenders[0];
            assert_eq!(c_str(first.label), "pizza");
            assert!(first.has_created_at);
            assert_eq!(first.created_at, CREATED_AT_MS);
            let second = &contenders[1];
            assert!(second.label.is_null());
            assert!(!second.has_created_at);
            assert_eq!(info.requested_label_count, 1);
            assert_eq!(
                (info.abstain_votes, info.lock_votes, info.end_time),
                (2, 5, 42)
            );
            dash_sdk_contest_info_free(Box::into_raw(Box::new(info)));
        }
    }

    #[test]
    fn should_list_requested_labels_once_in_contender_order() {
        let info = contest(&[
            (1, Some("pizza"), CREATED_AT_MS),
            (2, Some("p1zza"), CREATED_AT_MS + 1),
            (3, Some("pizza"), CREATED_AT_MS + 2),
        ]);
        unsafe {
            let labels =
                std::slice::from_raw_parts(info.requested_labels, info.requested_label_count)
                    .iter()
                    .map(|label| c_str(*label))
                    .collect::<Vec<_>>();
            assert_eq!(labels, ["pizza", "p1zza"]);
            dash_sdk_contest_info_free(Box::into_raw(Box::new(info)));
        }
    }
}
