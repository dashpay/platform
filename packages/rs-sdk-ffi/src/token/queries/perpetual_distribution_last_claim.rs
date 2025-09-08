//! Token perpetual distribution last claim query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Get token perpetual distribution last claim
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `token_id`: Base58-encoded token ID
/// - `identity_id`: Base58-encoded identity ID
///
/// # Returns
/// JSON string containing the last claim information
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_perpetual_distribution_last_claim(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
    identity_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || token_id.is_null() || identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, token ID, or identity ID is null".to_string(),
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

    let identity_id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let identity_id = match Identifier::from_string(identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        use dash_sdk::platform::query::TokenLastClaimQuery;
        use dash_sdk::platform::Fetch;

        let query = TokenLastClaimQuery {
            token_id: token_id,
            identity_id: identity_id,
        };

        let last_claim = RewardDistributionMoment::fetch(&wrapper.sdk, query)
            .await
            .map_err(|e| {
                FFIError::InternalError(format!(
                    "Failed to fetch token perpetual distribution last claim: {}",
                    e
                ))
            })?;

        // Convert RewardDistributionMoment to JSON
        match last_claim {
            Some(moment) => match moment {
                RewardDistributionMoment::TimeBasedMoment(ts) => Ok(format!(
                    r#"{{"type":"time_based","timestamp_ms":{},"block_height":0}}"#,
                    ts
                )),
                RewardDistributionMoment::BlockBasedMoment(height) => Ok(format!(
                    r#"{{"type":"block_based","timestamp_ms":0,"block_height":{}}}"#,
                    height
                )),
                RewardDistributionMoment::EpochBasedMoment(epoch) => Ok(format!(
                    r#"{{"type":"epoch_based","timestamp_ms":0,"block_height":{}}}"#,
                    epoch
                )),
            },
            None => Err(FFIError::NotFound("No last claim found".to_string())),
        }
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
