use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::{group_actions::GroupActionSignersQuery, Fetch};
use dpp::data_contract::group::GroupMemberPower;
use dpp::group::group_action_status::GroupActionStatus;
use drive_proof_verifier::types::groups::GroupActionSigners;
use std::ffi::{c_char, CStr, CString};

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
    let sdk = unsafe { &*sdk_handle }.sdk.clone();

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

        let contract_id = dash_sdk::Identifier::new(contract_id);
        let action_id = dash_sdk::Identifier::new(action_id);

        let status = match status {
            0 => GroupActionStatus::Pending,
            1 => GroupActionStatus::Completed,
            2 => GroupActionStatus::Expired,
            _ => return Err("Invalid status value".to_string()),
        };

        let query = GroupActionSignersQuery {
            contract_id,
            group_contract_position,
            status,
            action_id,
        };

        match GroupActionSigners::fetch(&sdk, query).await {
            Ok(Some(signers)) => {
                let signers_json: Vec<String> = signers
                    .signers()
                    .iter()
                    .map(|(id, power)| {
                        format!(
                            r#"{{"id":"{}","power":{}}}"#,
                            bs58::encode(id.as_bytes()).into_string(),
                            power
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", signers_json.join(","))))
            }
            Ok(None) => Ok(None),
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
