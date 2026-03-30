use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};
use std::ffi::CString;
use std::os::raw::c_void;

/// Fetches encrypted notes from the shielded pool, paginated.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_index` - Starting index (0-based) in the encrypted notes tree
/// * `count` - Maximum number of notes to return
///
/// # Returns
/// * JSON array of encrypted note objects, each with `cmx`, `nullifier`, `encryptedNote` (hex-encoded)
/// * Error if the operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - On success, returns a heap-allocated C string (JSON); caller must free with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_get_encrypted_notes(
    sdk_handle: *const SDKHandle,
    start_index: u64,
    count: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = wrapper.sdk.clone();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create Tokio runtime: {}", e),
            ));
        }
    };

    let query = ShieldedEncryptedNotesQuery { start_index, count };

    let result = rt.block_on(async move {
        ShieldedEncryptedNotes::fetch(&sdk, query)
            .await
            .map_err(|e| format!("Failed to fetch encrypted notes: {}", e))
    });

    match result {
        Ok(Some(notes)) => {
            let json_notes: Vec<serde_json::Value> = notes
                .0
                .iter()
                .map(|note| {
                    serde_json::json!({
                        "cmx": hex::encode(&note.cmx),
                        "nullifier": hex::encode(&note.nullifier),
                        "encryptedNote": hex::encode(&note.encrypted_note),
                    })
                })
                .collect();

            let json_str = serde_json::to_string(&json_notes).unwrap_or_else(|_| "[]".to_string());

            match CString::new(json_str) {
                Ok(c_str) => DashSDKResult {
                    data_type: DashSDKResultDataType::String,
                    data: c_str.into_raw() as *mut c_void,
                    error: std::ptr::null_mut(),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create CString: {}", e),
                )),
            }
        }
        Ok(None) => match CString::new("[]") {
            Ok(c_str) => DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            },
            Err(e) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            )),
        },
        Err(e) => DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e)),
    }
}
