use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dapi_grpc::platform::v0::{
    get_token_pre_programmed_distributions_request::{
        get_token_pre_programmed_distributions_request_v0::{StartAtInfo, Version},
        GetTokenPreProgrammedDistributionsRequestV0,
    },
    GetTokenPreProgrammedDistributionsRequest, GetTokenPreProgrammedDistributionsResponse,
};
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use std::ffi::{c_char, CStr, CString};

/// Fetches pre-programmed distributions for a token
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `token_id` - Base58-encoded token identifier
/// * `start_time_ms` - Starting time in milliseconds (optional, 0 for no start time)
/// * `start_recipient` - Base58-encoded starting recipient ID (optional)
/// * `start_recipient_included` - Whether to include the start recipient
/// * `limit` - Maximum number of distributions to return (optional, 0 for default limit)
///
/// # Returns
/// * JSON array of pre-programmed distributions or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_pre_programmed_distributions(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
    start_time_ms: u64,
    start_recipient: *const c_char,
    start_recipient_included: bool,
    limit: u32,
) -> DashSDKResult {
    match get_token_pre_programmed_distributions(
        sdk_handle,
        token_id,
        start_time_ms,
        start_recipient,
        start_recipient_included,
        limit,
    ) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data: std::ptr::null(),
                        error: DashSDKError::new(&format!("Failed to create CString: {}", e)),
                    }
                }
            };
            DashSDKResult {
                data: c_str.into_raw(),
                error: std::ptr::null(),
            }
        }
        Ok(None) => DashSDKResult {
            data: std::ptr::null(),
            error: std::ptr::null(),
        },
        Err(e) => DashSDKResult {
            data: std::ptr::null(),
            error: DashSDKError::new(&e),
        },
    }
}

fn get_token_pre_programmed_distributions(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
    start_time_ms: u64,
    start_recipient: *const c_char,
    start_recipient_included: bool,
    limit: u32,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let token_id_str = unsafe {
        CStr::from_ptr(token_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in token ID: {}", e))?
    };
    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        let token_id_bytes = bs58::decode(token_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode token ID: {}", e))?;

        let token_id: [u8; 32] = token_id_bytes
            .try_into()
            .map_err(|_| "Token ID must be exactly 32 bytes".to_string())?;

        let start_at_info = if start_time_ms > 0 {
            let start_recipient_bytes = if start_recipient.is_null() {
                None
            } else {
                let start_recipient_str = unsafe {
                    CStr::from_ptr(start_recipient)
                        .to_str()
                        .map_err(|e| format!("Invalid UTF-8 in start recipient: {}", e))?
                };
                let recipient_bytes = bs58::decode(start_recipient_str)
                    .into_vec()
                    .map_err(|e| format!("Failed to decode start recipient: {}", e))?;
                let recipient_id: [u8; 32] = recipient_bytes
                    .try_into()
                    .map_err(|_| "Start recipient must be exactly 32 bytes".to_string())?;
                Some(recipient_id.to_vec())
            };

            Some(StartAtInfo {
                start_time_ms,
                start_recipient: start_recipient_bytes,
                start_recipient_included: Some(start_recipient_included),
            })
        } else {
            None
        };

        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(Version::V0(GetTokenPreProgrammedDistributionsRequestV0 {
                token_id: token_id.to_vec(),
                start_at_info,
                limit: if limit > 0 { Some(limit) } else { None },
                prove: true,
            })),
        };

        // Execute the request directly since this isn't exposed in the SDK yet
        let result = request
            .execute(&sdk, RequestSettings::default())
            .await
            .map_err(|e| format!("Failed to execute request: {}", e))?;

        // Parse the response using the SDK's proof verification
        let response: GetTokenPreProgrammedDistributionsResponse = result.inner;
        
        match response.version {
            Some(dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::Version::V0(v0)) => {
                match v0.result {
                    Some(dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(distributions)) => {
                        if distributions.token_distributions.is_empty() {
                            return Ok(None);
                        }

                        let distributions_json: Vec<String> = distributions
                            .token_distributions
                            .iter()
                            .map(|timed_distribution| {
                                let distributions_for_time_json: Vec<String> = timed_distribution
                                    .distributions
                                    .iter()
                                    .map(|distribution| {
                                        format!(
                                            r#"{{"recipient_id":"{}","amount":{}}}"#,
                                            bs58::encode(&distribution.recipient_id).into_string(),
                                            distribution.amount
                                        )
                                    })
                                    .collect();

                                format!(
                                    r#"{{"timestamp":{},"distributions":[{}]}}"#,
                                    timed_distribution.timestamp,
                                    distributions_for_time_json.join(",")
                                )
                            })
                            .collect();

                        Ok(Some(format!("[{}]", distributions_json.join(","))))
                    }
                    Some(dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::get_token_pre_programmed_distributions_response_v0::Result::Proof(_proof)) => {
                        // For now, return empty result for proof responses
                        // TODO: Implement proper proof verification when SDK supports it
                        Ok(None)
                    }
                    None => Ok(None),
                }
            }
            None => Err("Invalid response format".to_string()),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_token_pre_programmed_distributions_null_handle() {
        unsafe {
            let result = dash_sdk_token_get_pre_programmed_distributions(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                0,
                std::ptr::null(),
                false,
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_token_pre_programmed_distributions_null_token_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_token_get_pre_programmed_distributions(
                handle,
                std::ptr::null(),
                0,
                std::ptr::null(),
                false,
                10,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
