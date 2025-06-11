//! Token contract info query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::tokens::contract_info::TokenContractInfo;
use dash_sdk::platform::Fetch;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Get token contract info
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `token_id`: Base58-encoded token ID
///
/// # Returns
/// JSON string containing the contract ID and token position, or null if not found
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_contract_info(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || token_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or token ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(token_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let token_id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid token ID: {}", e),
            ))
        }
    };

    let result: Result<Option<TokenContractInfo>, FFIError> = wrapper.runtime.block_on(async {
        // Fetch token contract info
        TokenContractInfo::fetch(&wrapper.sdk, token_id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(info)) => {
            // Create JSON representation
            use dash_sdk::dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
            let json_str = format!(
                "{{\"contract_id\":\"{}\",\"token_contract_position\":{}}}",
                info.contract_id().to_string(Encoding::Base58),
                info.token_contract_position()
            );

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
        Ok(None) => {
            // Return null for not found
            DashSDKResult::success_string(std::ptr::null_mut())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
