use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::{ShieldedNullifierStatuses, ShieldedNullifiersQuery};
use std::ffi::CString;
use std::os::raw::c_void;

/// Fetches nullifier statuses from the shielded pool.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `nullifiers_ptr` - Pointer to a contiguous array of 32-byte nullifier hashes
/// * `nullifiers_count` - Number of nullifiers (each is 32 bytes)
///
/// # Returns
/// * JSON array of objects with `nullifier` (hex) and `isSpent` (bool) fields
/// * Error if the operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - `nullifiers_ptr` must point to a valid byte array of length `nullifiers_count * 32`.
/// - On success, returns a heap-allocated C string (JSON); caller must free with `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_get_nullifiers(
    sdk_handle: *const SDKHandle,
    nullifiers_ptr: *const u8,
    nullifiers_count: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if nullifiers_ptr.is_null() || nullifiers_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Nullifiers pointer is null or count is zero".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = wrapper.sdk.clone();

    // Parse the raw byte pointer into Vec<[u8; 32]>
    let total_bytes = match (nullifiers_count as usize).checked_mul(32) {
        Some(n) => n,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Nullifiers count too large".to_string(),
            ));
        }
    };
    let raw_bytes = std::slice::from_raw_parts(nullifiers_ptr, total_bytes);
    let nullifiers: Vec<[u8; 32]> = raw_bytes
        .chunks_exact(32)
        .map(|chunk| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(chunk);
            arr
        })
        .collect();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create Tokio runtime: {}", e),
            ));
        }
    };

    let query = ShieldedNullifiersQuery(nullifiers);

    let result = rt.block_on(async move {
        ShieldedNullifierStatuses::fetch(&sdk, query)
            .await
            .map_err(|e| format!("Failed to fetch nullifier statuses: {}", e))
    });

    match result {
        Ok(Some(statuses)) => {
            let json_statuses: Vec<serde_json::Value> = statuses
                .0
                .iter()
                .map(|status| {
                    serde_json::json!({
                        "nullifier": hex::encode(status.nullifier),
                        "isSpent": status.is_spent,
                    })
                })
                .collect();

            let json_str =
                serde_json::to_string(&json_statuses).unwrap_or_else(|_| "[]".to_string());

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
