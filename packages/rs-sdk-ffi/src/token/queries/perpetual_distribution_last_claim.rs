//! Token perpetual distribution last claim query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::Fetch;
use dash_sdk::dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Query for token perpetual distribution last claim
#[derive(Debug)]
struct TokenPerpetualDistributionLastClaimQuery {
    token_id: Identifier,
    perpetual_distribution_position: u16,
}

impl
    dash_sdk::platform::Query<
        dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest,
    > for TokenPerpetualDistributionLastClaimQuery
{
    fn query(
        self,
        prove: bool,
    ) -> Result<
        dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest,
        dash_sdk::Error,
    > {
        use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_request::{
            GetTokenPerpetualDistributionLastClaimRequestV0, Version,
        };

        Ok(
            dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest {
                version: Some(Version::V0(
                    GetTokenPerpetualDistributionLastClaimRequestV0 {
                        token_id: self.token_id.to_vec(),
                        perpetual_distribution_position: self.perpetual_distribution_position
                            as u32,
                        prove,
                    },
                )),
            },
        )
    }
}

/// Get token perpetual distribution last claim
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `token_id`: Base58-encoded token ID
/// - `distribution_position`: Position of the perpetual distribution (0-based index)
///
/// # Returns
/// JSON string containing the last claim information
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_perpetual_distribution_last_claim(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
    distribution_position: u16,
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

    let result: Result<Option<RewardDistributionMoment>, FFIError> =
        wrapper.runtime.block_on(async {
            // Create the query
            let query = TokenPerpetualDistributionLastClaimQuery {
                token_id,
                perpetual_distribution_position: distribution_position,
            };

            // Fetch last claim
            RewardDistributionMoment::fetch(&wrapper.sdk, query)
                .await
                .map_err(FFIError::from)
        });

    match result {
        Ok(Some(moment)) => {
            // Create JSON representation
            let json_str = format!(
                "{{\"block_height\":{},\"core_block_height\":{},\"time_ms\":{}}}",
                moment.block_height, moment.core_block_height, moment.time_ms
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
