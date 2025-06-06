//! Multiple identities balance query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::IdentityBalance;
use dash_sdk::query_types::IdentityBalances;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch balances for multiple identities
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
///
/// # Returns
/// JSON string containing identity IDs mapped to their balances
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identities_fetch_balances(
    sdk_handle: *const SDKHandle,
    identity_ids: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || identity_ids.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or identity IDs is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let ids_str = match CStr::from_ptr(identity_ids).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse comma-separated identity IDs
    let identifiers: Result<Vec<Identifier>, DashSDKError> = ids_str
        .split(',')
        .map(|id_str| {
            Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid identity ID: {}", e),
                )
            })
        })
        .collect();

    let identifiers = match identifiers {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Fetch identities balances
        let balances: IdentityBalances = IdentityBalance::fetch_many(&wrapper.sdk, identifiers)
            .await
            .map_err(FFIError::from)?;

        // Convert to JSON string
        let mut json_parts = Vec::new();
        for (id, balance_opt) in balances {
            let balance_str = match balance_opt {
                Some(balance) => balance.to_string(),
                None => "null".to_string(),
            };
            json_parts.push(format!("\"{}\":{}", id, balance_str));
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
