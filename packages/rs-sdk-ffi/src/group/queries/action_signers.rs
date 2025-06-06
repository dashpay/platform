use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::data_contract::group::GroupMemberPower;
use dash_sdk::dpp::group::group_action_status::GroupActionStatus;
use dash_sdk::platform::{group_actions::GroupActionSignersQuery, FetchMany};
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches group action signers
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `group_contract_position` - Position of the group in the contract
/// * `status` - Action status (0=Pending, 1=Completed, 2=Expired)
/// * `action_id` - Base58-encoded action identifier
///
/// # Returns
/// * JSON array of signers or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_group_get_action_signers(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
    status: u8,
    action_id: *const c_char,
) -> DashSDKResult {
    match get_group_action_signers(
        sdk_handle,
        contract_id,
        group_contract_position,
        status,
        action_id,
    ) {
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

fn get_group_action_signers(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
    status: u8,
    action_id: *const c_char,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let contract_id_str = unsafe {
        CStr::from_ptr(contract_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contract ID: {}", e))?
    };
    let action_id_str = unsafe {
        CStr::from_ptr(action_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in action ID: {}", e))?
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

        let action_id_bytes = bs58::decode(action_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode action ID: {}", e))?;

        let action_id: [u8; 32] = action_id_bytes
            .try_into()
            .map_err(|_| "Action ID must be exactly 32 bytes".to_string())?;

        let contract_id = dash_sdk::platform::Identifier::new(contract_id);
        let action_id = dash_sdk::platform::Identifier::new(action_id);

        let status = match status {
            0 => GroupActionStatus::ActionActive,
            1 => GroupActionStatus::ActionClosed,
            _ => return Err("Invalid status value".to_string()),
        };

        let query = GroupActionSignersQuery {
            contract_id,
            group_contract_position,
            status,
            action_id,
        };

        match GroupMemberPower::fetch_many(&sdk, query).await {
            Ok(signers) => {
                if signers.is_empty() {
                    return Ok(None);
                }

                let signers_json: Vec<String> = signers
                    .iter()
                    .map(|(id, power_opt)| {
                        if let Some(power) = power_opt {
                            format!(
                                r#"{{"id":"{}","power":{}}}"#,
                                bs58::encode(id.as_bytes()).into_string(),
                                power
                            )
                        } else {
                            format!(
                                r#"{{"id":"{}","power":null}}"#,
                                bs58::encode(id.as_bytes()).into_string()
                            )
                        }
                    })
                    .collect();

                Ok(Some(format!("[{}]", signers_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch group action signers: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_group_action_signers_null_handle() {
        unsafe {
            let result = dash_sdk_group_get_action_signers(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                0,
                0,
                CString::new("test").unwrap().as_ptr(),
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_group_action_signers_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_group_get_action_signers(
                handle,
                std::ptr::null(),
                0,
                0,
                CString::new("test").unwrap().as_ptr(),
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
