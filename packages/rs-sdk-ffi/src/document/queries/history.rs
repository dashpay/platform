//! Document history query operations
//!
//! Reads the retained revisions of a document on a keep-history type
//! together with the authenticated lifecycle metadata: whether the document
//! is active, deleted, being erased, or absent, and how many revisions
//! remain.

use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::serialization::ValueConvertible;
use dash_sdk::platform::documents::document_history_query::{
    DocumentHistoryQuery, DocumentHistorySelector,
};
use dash_sdk::platform::Fetch;
use drive_proof_verifier::types::{DocumentHistory, DocumentHistoryState};
use drive_proof_verifier::ContextProvider;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Which revisions of a document's history to read
///
/// Exactly one selector applies to a `dash_sdk_document_fetch_history`
/// call; the selector decides which of `time_ms` and `revision` are read.
/// The variants carry a `History` prefix because cbindgen exports them as
/// bare C constants.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKDocumentHistorySelector {
    /// Revisions written at or after `time_ms`, oldest first. Pass 0 for
    /// the first page of a document's whole history.
    HistoryStartAtTime = 0,
    /// Revisions after the complete cursor (`time_ms`, `revision`) of the
    /// last entry received, for the pages after the first.
    HistoryStartAfter = 1,
    /// Revisions from history sequence `revision` onwards.
    HistoryStartAtRevision = 2,
    /// Exactly the revision at history sequence `revision`; the limit must
    /// be one.
    HistoryRevision = 3,
}

#[derive(Serialize)]
struct DocumentHistoryEntryJson {
    time_ms: u64,
    revision: u64,
    document: serde_json::Value,
}

#[derive(Serialize)]
struct DocumentHistoryLifecycleJson {
    state: &'static str,
    remaining_revisions: u64,
    deleted_at_ms: u64,
    erasing_started_at_ms: u64,
    erasing_from_time_ms: u64,
    erasing_from_revision: u64,
}

#[derive(Serialize)]
struct DocumentHistoryJson {
    entries: Vec<DocumentHistoryEntryJson>,
    lifecycle: DocumentHistoryLifecycleJson,
}

/// Serialize a verified history page as the JSON the FFI hands to callers.
///
/// Documents are the canonical query-side shape every other document read
/// returns (`$id` and `$ownerId` as base58 strings, `$formatVersion`
/// present). A history v1 response always authenticates lifecycle metadata;
/// a response without it came from a legacy protocol and is refused rather
/// than reported as a document with no lifecycle.
fn document_history_to_json(history: DocumentHistory) -> Result<String, FFIError> {
    let mut entries = Vec::with_capacity(history.entries.len());
    for entry in history.entries {
        let doc_value = entry.document.to_object().map_err(|e| {
            FFIError::InternalError(format!("Failed to convert document to JSON: {}", e))
        })?;
        let document = serde_json::to_value(&doc_value)
            .map_err(|e| FFIError::InternalError(format!("Failed to serialize document: {}", e)))?;
        entries.push(DocumentHistoryEntryJson {
            time_ms: entry.time_ms,
            revision: entry.revision,
            document,
        });
    }

    let lifecycle = history.lifecycle.ok_or_else(|| {
        FFIError::InternalError(
            "history response did not authenticate lifecycle metadata".to_string(),
        )
    })?;

    let result = DocumentHistoryJson {
        entries,
        lifecycle: DocumentHistoryLifecycleJson {
            state: match lifecycle.state {
                DocumentHistoryState::Active => "ACTIVE",
                DocumentHistoryState::Deleted => "DELETED",
                DocumentHistoryState::Erasing => "ERASING",
                DocumentHistoryState::Absent => "ABSENT",
            },
            remaining_revisions: lifecycle.remaining_revisions,
            deleted_at_ms: lifecycle.times.deleted_at_ms,
            erasing_started_at_ms: lifecycle.times.erasing_started_at_ms,
            erasing_from_time_ms: lifecycle.times.erasing_from_time_ms,
            erasing_from_revision: lifecycle.times.erasing_from_revision,
        },
    };

    serde_json::to_string(&result)
        .map_err(|e| FFIError::InternalError(format!("Failed to serialize result: {}", e)))
}

/// Fetch a page of a document's revision history with its lifecycle state
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `contract_id`: Base58-encoded contract ID (resolved through the trusted context provider)
/// - `document_type`: Name of a keep-history document type in that contract
/// - `document_id`: Base58-encoded document ID
/// - `selector`: Which revisions to read; decides how `time_ms` and `revision` are used
/// - `time_ms`: Inclusive lower time bound (`StartAtTime`) or cursor time (`StartAfter`)
/// - `revision`: Cursor revision (`StartAfter`), first revision (`StartAtRevision`), or the revision (`Revision`)
/// - `limit`: Maximum number of entries, at most ten (0 for the default)
///
/// # Returns
/// A JSON object:
/// `{"entries":[{"time_ms":…,"revision":…,"document":{…}}],"lifecycle":{"state":"ACTIVE"|"DELETED"|"ERASING"|"ABSENT","remaining_revisions":…,"deleted_at_ms":…,"erasing_started_at_ms":…,"erasing_from_time_ms":…,"erasing_from_revision":…}}`.
/// The lifecycle times are zero unless the state they describe has been
/// reached. An absent document yields no entries and the `ABSENT` state.
///
/// # Safety
/// - `sdk_handle`, `contract_id`, `document_type`, and `document_id` must be valid, non-null pointers.
/// - The three strings must be NUL-terminated and valid for the duration of the call.
/// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_fetch_history(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type: *const c_char,
    document_id: *const c_char,
    selector: DashSDKDocumentHistorySelector,
    time_ms: u64,
    revision: u64,
    limit: c_uint,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || contract_id.is_null()
        || document_type.is_null()
        || document_id.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let contract_id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_type_str = match CStr::from_ptr(document_type).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let document_id_str = match CStr::from_ptr(document_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let selector = match selector {
        DashSDKDocumentHistorySelector::HistoryStartAtTime => {
            DocumentHistorySelector::StartAtTime(time_ms)
        }
        DashSDKDocumentHistorySelector::HistoryStartAfter => {
            DocumentHistorySelector::StartAfter { time_ms, revision }
        }
        DashSDKDocumentHistorySelector::HistoryStartAtRevision => {
            DocumentHistorySelector::StartAtRevision(revision)
        }
        DashSDKDocumentHistorySelector::HistoryRevision => {
            DocumentHistorySelector::Revision(revision)
        }
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid contract ID: {}", e)))?;

        let document_id = Identifier::from_string(document_id_str, Encoding::Base58)
            .map_err(|e| FFIError::InternalError(format!("Invalid document ID: {}", e)))?;

        // Resolve the contract through the trusted context provider, as the
        // other document reads do, so the document type is checked before the
        // request goes out and the proof verifier finds the contract cached.
        let data_contract = if let Some(ref provider) = wrapper.trusted_provider {
            let platform_version = wrapper.sdk.version();
            provider
                .get_data_contract(&contract_id, platform_version)
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to get contract from context: {}", e))
                })?
                .ok_or_else(|| {
                    FFIError::InternalError(format!(
                        "Contract {} not found in trusted context",
                        contract_id_str
                    ))
                })?
        } else {
            return Err(FFIError::InternalError(
                "No trusted context provider configured".to_string(),
            ));
        };

        data_contract
            .document_type_for_name(document_type_str)
            .map_err(|e| FFIError::NotFound(format!("Document type not found: {}", e)))?;

        let query = DocumentHistoryQuery {
            data_contract_id: contract_id,
            document_type_name: document_type_str.to_string(),
            document_id,
            selector,
            limit: if limit == 0 { None } else { Some(limit) },
        };

        let history = DocumentHistory::fetch(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?
            .ok_or_else(|| {
                FFIError::InternalError("Document history response is missing".to_string())
            })?;

        document_history_to_json(history)
    });

    match result {
        Ok(json) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::*;
    use dash_sdk::dpp::document::{Document, DocumentV0};
    use dash_sdk::dpp::platform_value::Value;
    use drive_proof_verifier::types::{DocumentHistoryEntry, DocumentHistoryLifecycle};
    use std::collections::BTreeMap;
    use std::ptr;

    fn revision_document(revision: u64) -> Document {
        let mut properties = BTreeMap::new();
        properties.insert(
            "message".to_string(),
            Value::Text(format!("rev {revision}")),
        );
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::from([2u8; 32]),
            owner_id: Identifier::from([1u8; 32]),
            properties,
            revision: Some(revision),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    #[test]
    fn test_fetch_history_with_null_parameters() {
        let sdk_handle = create_mock_sdk_handle();
        let contract_id = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        let document_type = CString::new("note").unwrap();
        let document_id = CString::new("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF").unwrap();

        let null_cases: [(
            *const SDKHandle,
            *const c_char,
            *const c_char,
            *const c_char,
        ); 4] = [
            (
                ptr::null(),
                contract_id.as_ptr(),
                document_type.as_ptr(),
                document_id.as_ptr(),
            ),
            (
                sdk_handle,
                ptr::null(),
                document_type.as_ptr(),
                document_id.as_ptr(),
            ),
            (
                sdk_handle,
                contract_id.as_ptr(),
                ptr::null(),
                document_id.as_ptr(),
            ),
            (
                sdk_handle,
                contract_id.as_ptr(),
                document_type.as_ptr(),
                ptr::null(),
            ),
        ];

        for (handle, contract, doc_type, doc_id) in null_cases {
            let result = unsafe {
                dash_sdk_document_fetch_history(
                    handle,
                    contract,
                    doc_type,
                    doc_id,
                    DashSDKDocumentHistorySelector::HistoryStartAtTime,
                    0,
                    0,
                    0,
                )
            };
            assert!(!result.error.is_null());
            unsafe {
                let error = &*result.error;
                assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
            }
        }

        destroy_mock_sdk_handle(sdk_handle);
    }

    /// Two revisions written in one block keep their distinct cursors, the
    /// document is the canonical query-side shape, and every lifecycle time
    /// crosses the boundary exactly.
    #[test]
    fn test_history_json_keeps_cursors_lifecycle_and_canonical_documents() {
        use dash_sdk::drive::drive::document::history::DocumentHistoryLifecycleTimes;

        let history = DocumentHistory {
            entries: (1..=2)
                .map(|revision| DocumentHistoryEntry {
                    time_ms: 1_700_000_000_000,
                    revision,
                    document: revision_document(revision),
                })
                .collect(),
            lifecycle: Some(DocumentHistoryLifecycle {
                state: DocumentHistoryState::Erasing,
                remaining_revisions: 2,
                times: DocumentHistoryLifecycleTimes {
                    deleted_at_ms: 1_700_000_000_001,
                    erasing_started_at_ms: 1_700_000_000_002,
                    erasing_from_time_ms: 1_700_000_000_000,
                    erasing_from_revision: 2,
                },
            }),
        };

        let json: serde_json::Value =
            serde_json::from_str(&document_history_to_json(history).unwrap()).unwrap();

        let entries = json["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0]["time_ms"],
            serde_json::json!(1_700_000_000_000u64)
        );
        assert_eq!(entries[0]["revision"], serde_json::json!(1));
        assert_eq!(entries[1]["revision"], serde_json::json!(2));
        assert_eq!(entries[1]["document"]["$revision"], serde_json::json!(2));
        assert_eq!(
            entries[1]["document"]["message"],
            serde_json::json!("rev 2")
        );
        assert!(entries[1]["document"]["$id"].is_string());
        assert!(entries[1]["document"]["$ownerId"].is_string());

        let lifecycle = &json["lifecycle"];
        assert_eq!(lifecycle["state"], serde_json::json!("ERASING"));
        assert_eq!(lifecycle["remaining_revisions"], serde_json::json!(2));
        assert_eq!(
            lifecycle["deleted_at_ms"],
            serde_json::json!(1_700_000_000_001u64)
        );
        assert_eq!(
            lifecycle["erasing_started_at_ms"],
            serde_json::json!(1_700_000_000_002u64)
        );
        assert_eq!(
            lifecycle["erasing_from_time_ms"],
            serde_json::json!(1_700_000_000_000u64)
        );
        assert_eq!(lifecycle["erasing_from_revision"], serde_json::json!(2));
    }

    /// An absent document reports the `ABSENT` state with no entries; a
    /// response without lifecycle metadata is a legacy response and refused.
    #[test]
    fn test_history_json_reports_absent_and_refuses_legacy_responses() {
        let absent = DocumentHistory {
            entries: vec![],
            lifecycle: Some(DocumentHistoryLifecycle {
                state: DocumentHistoryState::Absent,
                remaining_revisions: 0,
                times: Default::default(),
            }),
        };
        let json: serde_json::Value =
            serde_json::from_str(&document_history_to_json(absent).unwrap()).unwrap();
        assert_eq!(json["entries"].as_array().unwrap().len(), 0);
        assert_eq!(json["lifecycle"]["state"], serde_json::json!("ABSENT"));
        assert_eq!(
            json["lifecycle"]["remaining_revisions"],
            serde_json::json!(0)
        );
        assert_eq!(json["lifecycle"]["deleted_at_ms"], serde_json::json!(0));

        let legacy = DocumentHistory {
            entries: vec![],
            lifecycle: None,
        };
        assert!(document_history_to_json(legacy).is_err());
    }
}
