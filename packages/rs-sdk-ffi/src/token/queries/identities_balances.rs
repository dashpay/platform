//! Multiple identities token balances query operations

use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::identity_token_balances::IdentitiesTokenBalancesQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::identity_token_balance::IdentitiesTokenBalances;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch token balances for multiple identities for a specific token
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Either a comma-separated list OR a JSON array of Base58-encoded identity IDs
/// - `token_id`: Base58-encoded token ID
///
/// # Returns
/// JSON string containing identity IDs mapped to their token balances
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identities_fetch_token_balances(
    sdk_handle: *const SDKHandle,
    identity_ids: *const c_char,
    token_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || identity_ids.is_null() || token_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, identity IDs, or token ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let ids_str = match CStr::from_ptr(identity_ids).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let token_str = match CStr::from_ptr(token_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse identity IDs: accept JSON array ["id1","id2"] or comma-separated "id1,id2"
    let identity_ids: Result<Vec<Identifier>, DashSDKError> =
        if ids_str.trim_start().starts_with('[') {
            // JSON array
            let arr: Result<Vec<String>, _> = serde_json::from_str(ids_str);
            match arr {
                Ok(items) => items
                    .into_iter()
                    .map(|id_str| {
                        Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                            DashSDKError::new(
                                DashSDKErrorCode::InvalidParameter,
                                format!("Invalid identity ID: {}", e),
                            )
                        })
                    })
                    .collect(),
                Err(e) => Err(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid identity IDs JSON: {}", e),
                )),
            }
        } else {
            // Comma-separated
            ids_str
                .split(',')
                .map(|id_str| {
                    Identifier::from_string(id_str.trim(), Encoding::Base58).map_err(|e| {
                        DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            format!("Invalid identity ID: {}", e),
                        )
                    })
                })
                .collect()
        };

    let identity_ids = match identity_ids {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    let token_id = match Identifier::from_string(token_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid token ID: {}", e),
            ))
        }
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Create the query
        let query = IdentitiesTokenBalancesQuery {
            identity_ids,
            token_id,
        };

        // Fetch token balances
        let balances: IdentitiesTokenBalances = TokenAmount::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?;

        // Convert to JSON string
        let mut json_parts = Vec::new();
        for (identity_id, balance_opt) in balances.0.iter() {
            let balance_str = match balance_opt {
                Some(balance) => {
                    let val: &u64 = balance;
                    val.to_string()
                }
                None => "null".to_string(),
            };
            json_parts.push(format!(
                "\"{}\":{}",
                identity_id.to_string(Encoding::Base58),
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
