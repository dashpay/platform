use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchMany;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use std::ffi::{c_char, CStr, CString};

/// Fetches information about multiple groups
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_at_position` - Starting position (optional, null for beginning)
/// * `limit` - Maximum number of groups to return
///
/// # Returns
/// * JSON array of group information or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_group_get_infos(
    sdk_handle: *const SDKHandle,
    start_at_position: *const c_char,
    limit: u32,
) -> DashSDKResult {
    match get_group_infos(sdk_handle, start_at_position, limit) {
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

fn get_group_infos(
    sdk_handle: *const SDKHandle,
    start_at_position: *const c_char,
    limit: u32,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        let start_position: GroupContractPosition = if start_at_position.is_null() {
            0
        } else {
            let position_str = unsafe {
                CStr::from_ptr(start_at_position)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start position: {}", e))?
            };
            position_str
                .parse::<u16>()
                .map_err(|e| format!("Failed to parse start position: {}", e))?
        };

        let query = dash_sdk::platform::LimitQuery {
            query: start_position,
            limit: Some(limit),
            start_info: None,
        };

        match Group::fetch_many(&sdk, query).await {
            Ok(groups) => {
                if groups.is_empty() {
                    return Ok(None);
                }

                let groups_json: Vec<String> = groups
                    .values()
                    .map(|group| {
                        let members_json: Vec<String> = group
                            .members()
                            .iter()
                            .map(|(id, power)| {
                                format!(
                                    r#"{{"id":"{}","power":{}}}"#,
                                    bs58::encode(id.as_bytes()).into_string(),
                                    power
                                )
                            })
                            .collect();

                        format!(
                            r#"{{"required_power":{},"members":[{}]}}"#,
                            group.required_power(),
                            members_json.join(",")
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", groups_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch group infos: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_group_infos_null_handle() {
        unsafe {
            let result = dash_sdk_group_get_infos(std::ptr::null(), std::ptr::null(), 10);
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_group_infos() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_group_get_infos(handle, std::ptr::null(), 10);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
