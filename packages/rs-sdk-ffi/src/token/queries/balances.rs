//! Token balance query operations

use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::identity_token_balance::IdentityTokenBalances;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Get identity token balances
///
/// This is an alias for dash_sdk_identity_fetch_token_balances for backward compatibility
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_id`: Base58-encoded identity ID
/// - `token_ids`: Comma-separated list of Base58-encoded token IDs
///
/// # Returns
/// JSON string containing token IDs mapped to their balances
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_identity_balances(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    token_ids: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || identity_id.is_null() || token_ids.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, identity ID, or token IDs is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let tokens_str = match CStr::from_ptr(token_ids).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let identity_id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    // Parse comma-separated token IDs
    let token_ids: Result<Vec<Identifier>, DashSDKError> = tokens_str
        .split(',')
        .map(|id_str| {
            Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid token ID: {}", e),
                )
            })
        })
        .collect();

    let token_ids = match token_ids {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Create the query
        let query = IdentityTokenBalancesQuery {
            identity_id,
            token_ids,
        };

        // Fetch token balances
        let balances: IdentityTokenBalances = TokenAmount::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?;

        // Convert to JSON string
        let mut json_parts = Vec::new();
        for (token_id, balance_opt) in balances.0.iter() {
            let balance_str = match balance_opt {
                Some(balance) => {
                    let val: &u64 = balance;
                    val.to_string()
                }
                None => "null".to_string(),
            };
            json_parts.push(format!(
                "\"{}\":{}",
                token_id.to_string(Encoding::Base58),
                balance_str
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
