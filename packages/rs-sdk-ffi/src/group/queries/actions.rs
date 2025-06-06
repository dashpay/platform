use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::{group_actions::GroupActionsQuery, Fetch};
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use drive_proof_verifier::types::groups::GroupActions;
use std::ffi::{c_char, CStr, CString};

/// Fetches group actions
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `group_contract_position` - Position of the group in the contract
/// * `status` - Action status (0=Pending, 1=Completed, 2=Expired)
/// * `start_at_action_id` - Optional starting action ID (Base58-encoded)
/// * `limit` - Maximum number of actions to return
///
/// # Returns
/// * JSON array of group actions or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_group_get_actions(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
    status: u8,
    start_at_action_id: *const c_char,
    limit: u16,
) -> DashSDKResult {
    match get_group_actions(
        sdk_handle,
        contract_id,
        group_contract_position,
        status,
        start_at_action_id,
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

fn get_group_actions(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    group_contract_position: u16,
    status: u8,
    start_at_action_id: *const c_char,
    limit: u16,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let contract_id_str = unsafe {
        CStr::from_ptr(contract_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contract ID: {}", e))?
    };
    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        let contract_id_bytes = bs58::decode(contract_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode contract ID: {}", e))?;

        let contract_id: [u8; 32] = contract_id_bytes
            .try_into()
            .map_err(|_| "Contract ID must be exactly 32 bytes".to_string())?;

        let contract_id = dash_sdk::Identifier::new(contract_id);

        let status = match status {
            0 => GroupActionStatus::Pending,
            1 => GroupActionStatus::Completed,
            2 => GroupActionStatus::Expired,
            _ => return Err("Invalid status value".to_string()),
        };

        let start_at_action_id = if start_at_action_id.is_null() {
            None
        } else {
            let action_id_str = unsafe {
                CStr::from_ptr(start_at_action_id)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start action ID: {}", e))?
            };
            let action_id_bytes = bs58::decode(action_id_str)
                .into_vec()
                .map_err(|e| format!("Failed to decode start action ID: {}", e))?;
            let action_id: [u8; 32] = action_id_bytes
                .try_into()
                .map_err(|_| "Action ID must be exactly 32 bytes".to_string())?;
            Some((dash_sdk::Identifier::new(action_id), true))
        };

        let query = GroupActionsQuery {
            contract_id,
            group_contract_position,
            status,
            start_at_action_id,
            limit: Some(limit),
        };

        match GroupActions::fetch(&sdk, query).await {
            Ok(Some(actions)) => {
                let actions_json: Vec<String> = actions
                    .actions()
                    .iter()
                    .map(|action| {
                        format!(
                            r#"{{"id":"{}","proposal_id":"{}","proposal_owner_id":"{}","group_contract_position":{},"action_type":"{}","date_proposed":{}}}"#,
                            bs58::encode(action.id().as_bytes()).into_string(),
                            bs58::encode(action.proposal_id().as_bytes()).into_string(),
                            bs58::encode(action.proposal_owner_id().as_bytes()).into_string(),
                            action.group_contract_position(),
                            format!("{:?}", action.action_type()),
                            action.date_proposed()
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", actions_json.join(","))))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to fetch group actions: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_group_actions_null_handle() {
        unsafe {
            let result = dash_sdk_group_get_actions(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                0,
                0,
                std::ptr::null(),
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_group_actions_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result =
                dash_sdk_group_get_actions(handle, std::ptr::null(), 0, 0, std::ptr::null(), 10);
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
