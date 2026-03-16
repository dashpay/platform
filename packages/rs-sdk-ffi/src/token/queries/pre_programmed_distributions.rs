//! Token pre-programmed distributions query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::tokens::token_pre_programmed_distributions::{
    TokenPreProgrammedDistributions, TokenPreProgrammedDistributionsQuery,
    TokenPreProgrammedDistributionsStartAtInfo,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetches pre-programmed distributions for a token.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `token_id` - Base58-encoded token identifier
/// * `start_time_ms` - Starting time in milliseconds (optional, 0 for no start time)
/// * `start_recipient` - Base58-encoded starting recipient ID (optional, null for none)
/// * `start_recipient_included` - Whether to include the start recipient
/// * `limit` - Maximum number of distributions to return (optional, 0 for default limit)
///
/// # Returns
/// * JSON array of pre-programmed distributions or null if not found
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
/// - `token_id` must be a valid pointer to a NUL-terminated C string.
/// - `start_recipient` may be null; if non-null, must be a valid NUL-terminated C string.
/// - The returned C string pointer (on success) must be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_pre_programmed_distributions(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
    start_time_ms: u64,
    start_recipient: *const c_char,
    start_recipient_included: bool,
    limit: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() || token_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or token ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let token_id_str = match CStr::from_ptr(token_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let token_id = match Identifier::from_string(token_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid token ID: {}", e),
            ))
        }
    };

    let start_at_info = if start_time_ms > 0 {
        let recipient = if !start_recipient.is_null() {
            let recipient_str = match CStr::from_ptr(start_recipient).to_str() {
                Ok(s) => s,
                Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
            };
            match Identifier::from_string(recipient_str, Encoding::Base58) {
                Ok(id) => Some(id),
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid start recipient: {}", e),
                    ))
                }
            }
        } else {
            None
        };

        Some(TokenPreProgrammedDistributionsStartAtInfo {
            start_time_ms,
            start_recipient: recipient,
            start_recipient_included,
        })
    } else {
        if !start_recipient.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "start_recipient provided but start_time_ms is zero".to_string(),
            ));
        }
        None
    };

    let query = TokenPreProgrammedDistributionsQuery {
        token_id,
        start_at_info,
        limit: if limit > 0 { Some(limit) } else { None },
    };

    let result: Result<Option<String>, FFIError> = wrapper.runtime.block_on(async {
        use dash_sdk::platform::Fetch;

        let distributions = TokenPreProgrammedDistributions::fetch(&wrapper.sdk, query)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!(
                    "Failed to fetch token pre-programmed distributions: {}",
                    e
                ))
            })?;

        match distributions {
            Some(dists) => {
                let distributions_json: Vec<String> = dists
                    .0
                    .iter()
                    .map(|(timestamp, recipients)| {
                        let recipients_json: Vec<String> = recipients
                            .iter()
                            .map(|(recipient_id, amount)| {
                                format!(
                                    r#"{{"recipient_id":"{}","amount":{}}}"#,
                                    recipient_id, amount
                                )
                            })
                            .collect();

                        format!(
                            r#"{{"timestamp":{},"distributions":[{}]}}"#,
                            timestamp,
                            recipients_json.join(",")
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", distributions_json.join(","))))
            }
            None => Ok(None),
        }
    });

    match result {
        Ok(Some(json_str)) => {
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
        Ok(None) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_null_handle() {
        unsafe {
            let token_id = std::ffi::CString::new("test").unwrap();
            let result = dash_sdk_token_get_pre_programmed_distributions(
                std::ptr::null(),
                token_id.as_ptr(),
                0,
                std::ptr::null(),
                false,
                0,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_null_token_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_token_get_pre_programmed_distributions(
                handle,
                std::ptr::null(),
                0,
                std::ptr::null(),
                false,
                0,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
