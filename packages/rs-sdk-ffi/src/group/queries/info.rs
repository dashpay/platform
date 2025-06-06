use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::data_contract::group::Group;
use dash_sdk::platform::{group_actions::GroupQuery, Fetch};
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches information about a group
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `group_contract_position` - Position of the group in the contract
///
/// # Returns
/// * JSON string with group information or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_group_get_info(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
) -> DashSDKResult {
    match get_group_info(sdk_handle, contract_id, group_contract_position) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::None,
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
            data_type: DashSDKResultDataType::None,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::None,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

fn get_group_info(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    if contract_id.is_null() {
        return Err("Contract ID is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let contract_id_str = unsafe {
        CStr::from_ptr(contract_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contract ID: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let contract_id_bytes = bs58::decode(contract_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode contract ID: {}", e))?;

        let contract_id: [u8; 32] = contract_id_bytes
            .try_into()
            .map_err(|_| "Contract ID must be exactly 32 bytes".to_string())?;

        let contract_id = dash_sdk::platform::Identifier::new(contract_id);

        let query = GroupQuery {
            contract_id,
            group_contract_position,
        };

        match Group::fetch(&sdk, query).await {
            Ok(Some(group)) => {
                // Convert members to JSON based on group variant
                let (members, required_power) = match &group {
                    Group::V0(v0) => (&v0.members, v0.required_power),
                };

                let members_json: Vec<String> = members
                    .iter()
                    .map(|(id, power)| {
                        format!(
                            r#"{{"id":"{}","power":{}}}"#,
                            bs58::encode(id.as_bytes()).into_string(),
                            power
                        )
                    })
                    .collect();

                let json = format!(
                    r#"{{"required_power":{},"members":[{}]}}"#,
                    required_power,
                    members_json.join(",")
                );
                Ok(Some(json))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to fetch group info: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_group_info_null_handle() {
        unsafe {
            let result = dash_sdk_group_get_info(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                0,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_group_info_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_group_get_info(handle, std::ptr::null(), 0);
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
