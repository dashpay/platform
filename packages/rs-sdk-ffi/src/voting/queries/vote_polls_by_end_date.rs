use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::drive::query::VotePollsByEndDateDriveQuery;
use dash_sdk::platform::FetchMany;
use std::ffi::{c_void, CString};

/// Fetches vote polls by end date
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_time_ms` - Start time in milliseconds (optional, 0 for no start time)
/// * `start_time_included` - Whether to include the start time
/// * `end_time_ms` - End time in milliseconds (optional, 0 for no end time)
/// * `end_time_included` - Whether to include the end time
/// * `limit` - Maximum number of results to return (optional, 0 for no limit)
/// * `offset` - Number of results to skip (optional, 0 for no offset)
/// * `ascending` - Whether to order results in ascending order
///
/// # Returns
/// * JSON array of vote polls grouped by timestamp or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_voting_get_vote_polls_by_end_date(
    sdk_handle: *const SDKHandle,
    start_time_ms: u64,
    start_time_included: bool,
    end_time_ms: u64,
    end_time_included: bool,
    limit: u32,
    offset: u32,
    ascending: bool,
) -> DashSDKResult {
    match get_vote_polls_by_end_date(
        sdk_handle,
        start_time_ms,
        start_time_included,
        end_time_ms,
        end_time_included,
        limit,
        offset,
        ascending,
    ) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::NoData,
                        data: std::ptr::null_mut(),
                        error: Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to create CString: {}", e),
                        ))),
                    }
                }
            };
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn get_vote_polls_by_end_date(
    sdk_handle: *const SDKHandle,
    start_time_ms: u64,
    start_time_included: bool,
    end_time_ms: u64,
    end_time_included: bool,
    limit: u32,
    offset: u32,
    ascending: bool,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let start_time_info = if start_time_ms > 0 {
            Some((start_time_ms, start_time_included))
        } else {
            None
        };

        let end_time_info = if end_time_ms > 0 {
            Some((end_time_ms, end_time_included))
        } else {
            None
        };

        let query = VotePollsByEndDateDriveQuery {
            start_time: start_time_info,
            end_time: end_time_info,
            limit: if limit > 0 { Some(limit as u16) } else { None },
            offset: if offset > 0 {
                Some(offset as u16)
            } else {
                None
            },
            order_ascending: ascending,
        };

        match VotePoll::fetch_many(&sdk, query).await {
            Ok(vote_polls_grouped) => {
                if vote_polls_grouped.0.is_empty() {
                    return Ok(None);
                }

                let grouped_json: Vec<String> = vote_polls_grouped
                    .0
                    .iter()
                    .map(|(timestamp, vote_polls)| {
                        let polls_json: Vec<String> = vote_polls
                            .iter()
                            .map(|_poll| format!(r#"{{"end_time":{}}}"#, timestamp))
                            .collect();

                        format!(
                            r#"{{"timestamp":{},"vote_polls":[{}]}}"#,
                            timestamp,
                            polls_json.join(",")
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", grouped_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch vote polls by end date: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_vote_polls_by_end_date_null_handle() {
        unsafe {
            let result = dash_sdk_voting_get_vote_polls_by_end_date(
                std::ptr::null(),
                0,
                false,
                0,
                false,
                10,
                0,
                true,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_vote_polls_by_end_date() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let _result =
                dash_sdk_voting_get_vote_polls_by_end_date(handle, 0, false, 0, false, 10, 0, true);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
