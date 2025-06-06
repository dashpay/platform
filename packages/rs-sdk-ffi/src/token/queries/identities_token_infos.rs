//! Multiple identities token infos query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::tokens::info::{v0::IdentityTokenInfoV0Accessors, IdentityTokenInfo};
use dash_sdk::platform::tokens::token_info::IdentitiesTokenInfosQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::token_info::IdentitiesTokenInfos;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch token information for multiple identities for a specific token
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Comma-separated list of Base58-encoded identity IDs
/// - `token_id`: Base58-encoded token ID
///
/// # Returns
/// JSON string containing identity IDs mapped to their token information
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identities_fetch_token_infos(
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

    // Parse comma-separated identity IDs
    let identity_ids: Result<Vec<Identifier>, DashSDKError> = ids_str
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
        let query = IdentitiesTokenInfosQuery {
            identity_ids,
            token_id,
        };

        // Fetch token infos
        let token_infos: IdentitiesTokenInfos = IdentityTokenInfo::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?;

        // Convert to JSON string
        let mut json_parts = Vec::new();
        for (identity_id, info_opt) in token_infos.0.iter() {
            let info_json = match info_opt {
                Some(info) => {
                    // Create JSON representation of IdentityTokenInfo
                    format!(
                        "{{\"frozen\":{}}}",
                        if info.frozen() { "true" } else { "false" }
                    )
                }
                None => "null".to_string(),
            };
            json_parts.push(format!(
                "\"{}\":{}",
                identity_id.to_string(Encoding::Base58),
                info_json
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
