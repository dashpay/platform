//! Multiple data contracts query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::{DataContract, FetchMany};
use dash_sdk::query_types::DataContracts;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch multiple data contracts by their IDs
///
/// # Safety
/// - `sdk_handle` and `contract_ids` must be valid, non-null pointers.
/// - `contract_ids` must point to a NUL-terminated C string containing either a JSON array of Base58 IDs or a comma-separated list; it must remain valid for the duration of the call.
/// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `contract_ids`: Comma-separated list of Base58-encoded contract IDs
///
/// # Returns
/// JSON string containing contract IDs mapped to their data contracts
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contracts_fetch_many(
    sdk_handle: *const SDKHandle,
    contract_ids: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || contract_ids.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or contract IDs is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let ids_str = match CStr::from_ptr(contract_ids).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Accept either a JSON array of strings or a comma-separated list
    let identifiers: Result<Vec<Identifier>, DashSDKError> =
        if ids_str.trim_start().starts_with('[') {
            match serde_json::from_str::<Vec<String>>(ids_str) {
                Ok(list) => list
                    .into_iter()
                    .map(|s| {
                        Identifier::from_string(s.as_str(), Encoding::Base58).map_err(|e| {
                            DashSDKError::new(
                                DashSDKErrorCode::InvalidParameter,
                                format!("Invalid contract ID: {}", e),
                            )
                        })
                    })
                    .collect(),
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid JSON array of IDs: {}", e),
                    ))
                }
            }
        } else {
            ids_str
                .split(',')
                .map(|id_str| {
                    Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                        DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            format!("Invalid contract ID: {}", e),
                        )
                    })
                })
                .collect()
        };

    let identifiers = match identifiers {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Fetch data contracts
        let contracts: DataContracts = DataContract::fetch_many(&wrapper.sdk, identifiers)
            .await
            .map_err(FFIError::from)?;

        // Convert to JSON string
        let mut json_parts = Vec::new();
        for (id, contract_opt) in contracts {
            let contract_json = match contract_opt {
                Some(contract) => {
                    serde_json::to_string(&contract).unwrap_or_else(|_| "null".to_string())
                }
                None => "null".to_string(),
            };
            json_parts.push(format!(
                "\"{}\":{}",
                id.to_string(Encoding::Base58),
                contract_json
            ));
        }

        Ok(format!("{{{}}}", json_parts.join(",")))
    });

    match result {
        Ok(json_str) => {
            let c_str = match CString::new(json_str) {
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
