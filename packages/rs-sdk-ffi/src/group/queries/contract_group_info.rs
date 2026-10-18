//! Proved read of a contract group's stored information: its owner, its admins, its name and
//! its description. Wraps [`ContractGroupInfo::fetch`] from `dash_sdk::platform::contract_groups`.

use crate::sdk::SDKWrapper;
use crate::{DashSDKError, DashSDKResult, DashSDKResultDataType, FFIError, SDKHandle};
use dash_sdk::platform::contract_groups::ContractGroupInfo;
use dash_sdk::platform::{Fetch, Identifier};
use serde::Serialize;
use std::ffi::{c_char, CStr, CString};

/// The JSON shape of a contract group's information, matching the wasm SDK's
/// `ContractGroupInfo.toJSON()`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractGroupInfoJson {
    owner_id: String,
    admin_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl From<&ContractGroupInfo> for ContractGroupInfoJson {
    fn from(info: &ContractGroupInfo) -> Self {
        let owner = info.owner();
        ContractGroupInfoJson {
            owner_id: bs58::encode(owner.owner_id().as_bytes()).into_string(),
            admin_ids: owner
                .admin_ids()
                .map(|admins| {
                    admins
                        .iter()
                        .map(|admin| bs58::encode(admin.as_bytes()).into_string())
                        .collect()
                })
                .unwrap_or_default(),
            name: info.name().map(str::to_string),
            description: info.description().map(str::to_string),
        }
    }
}

/// Fetches, with a proof, the stored information of a contract group.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_group_id` - Base58-encoded contract group identifier
///
/// # Returns
/// * A JSON string `{"ownerId":"<base58>","adminIds":["<base58>",...],"name":"...","description":"..."}`.
///   `adminIds` is empty for a group with a single owner; `name` and `description` are
///   omitted when the group has none.
/// * No data and no error when no such group exists.
/// * An error when an argument is invalid or the fetch fails.
///
/// # Safety
/// - `sdk_handle` and `contract_group_id` must be valid, non-null pointers.
/// - `contract_group_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contract_group_get_info(
    sdk_handle: *const SDKHandle,
    contract_group_id: *const c_char,
) -> DashSDKResult {
    match get_contract_group_info(sdk_handle, contract_group_id) {
        Ok(Some(json)) => match CString::new(json) {
            Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
            Err(e) => DashSDKResult::error(FFIError::from(e).into()),
        },
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult::error(DashSDKError::from(e)),
    }
}

#[allow(clippy::result_large_err)]
unsafe fn get_contract_group_info(
    sdk_handle: *const SDKHandle,
    contract_group_id: *const c_char,
) -> Result<Option<String>, FFIError> {
    if sdk_handle.is_null() {
        return Err(FFIError::InvalidParameter("SDK handle is null".to_string()));
    }
    if contract_group_id.is_null() {
        return Err(FFIError::InvalidParameter(
            "Contract group ID is null".to_string(),
        ));
    }

    let id_str = CStr::from_ptr(contract_group_id).to_str()?;
    let id_bytes: [u8; 32] = bs58::decode(id_str)
        .into_vec()
        .map_err(|e| {
            FFIError::InvalidParameter(format!("Failed to decode contract group ID: {}", e))
        })?
        .try_into()
        .map_err(|_| {
            FFIError::InvalidParameter("Contract group ID must be exactly 32 bytes".to_string())
        })?;

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = wrapper.sdk.clone();

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| FFIError::InternalError(format!("Failed to create Tokio runtime: {}", e)))?;

    rt.block_on(async move {
        let info = ContractGroupInfo::fetch(&sdk, Identifier::new(id_bytes)).await?;
        info.map(|info| {
            serde_json::to_string(&ContractGroupInfoJson::from(&info)).map_err(FFIError::from)
        })
        .transpose()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::DashSDKErrorCode;
    use dash_sdk::dpp::contract_group::{ContractGroupInfoV0, ContractGroupOwner};
    use std::collections::BTreeSet;

    const GROUP_ID: [u8; 32] = [3u8; 32];
    const OWNER_ID: [u8; 32] = [1u8; 32];
    const ADMIN_ID: [u8; 32] = [2u8; 32];

    fn base58(bytes: [u8; 32]) -> String {
        bs58::encode(bytes).into_string()
    }

    /// Mock SDK handle that answers `ContractGroupInfo::fetch(GROUP_ID)` with `info`.
    fn create_mock_sdk_handle_with_group(info: Option<ContractGroupInfo>) -> *mut SDKHandle {
        let mut wrapper = Box::new(SDKWrapper::new_mock());

        // Registering expectations is async, and the FFI entry point below spins
        // up its own runtime and blocks on it, so this runtime must be gone by
        // the time it runs.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(async {
            wrapper
                .sdk
                .mock()
                .expect_fetch::<ContractGroupInfo, _>(Identifier::new(GROUP_ID), info)
                .await
                .expect("register contract group info expectation");
        });
        drop(setup_runtime);

        Box::into_raw(wrapper) as *mut SDKHandle
    }

    /// Calls the entry point and returns the error code it reports, freeing the result.
    unsafe fn error_code_of(result: DashSDKResult) -> DashSDKErrorCode {
        assert!(result.data.is_null());
        assert!(!result.error.is_null(), "the call must fail");
        let code = (*result.error).code;
        crate::dash_sdk_error_free(result.error);
        code
    }

    unsafe fn json_of(result: DashSDKResult) -> serde_json::Value {
        assert!(result.error.is_null(), "the call must succeed");
        assert_eq!(result.data_type, DashSDKResultDataType::String);
        let json = CString::from_raw(result.data as *mut c_char)
            .into_string()
            .expect("utf-8 json");
        serde_json::from_str(&json).expect("valid json")
    }

    #[test]
    fn test_get_contract_group_info_null_handle() {
        let id = CString::new(base58(GROUP_ID)).unwrap();
        unsafe {
            let result = dash_sdk_contract_group_get_info(std::ptr::null(), id.as_ptr());
            assert_eq!(error_code_of(result), DashSDKErrorCode::InvalidParameter);
        }
    }

    #[test]
    fn test_get_contract_group_info_null_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contract_group_get_info(handle, std::ptr::null());
            assert_eq!(error_code_of(result), DashSDKErrorCode::InvalidParameter);
        }
        destroy_mock_sdk_handle(handle);
    }

    #[test]
    fn test_get_contract_group_info_rejects_malformed_ids() {
        let handle = create_mock_sdk_handle();
        for bad in ["not base58 0OIl", &base58([0u8; 32])[..10]] {
            let id = CString::new(bad).unwrap();
            unsafe {
                let result = dash_sdk_contract_group_get_info(handle, id.as_ptr());
                assert_eq!(
                    error_code_of(result),
                    DashSDKErrorCode::InvalidParameter,
                    "{bad:?} must be refused before any request"
                );
            }
        }
        destroy_mock_sdk_handle(handle);
    }

    /// A group with admins, a name and no description reports its owner, its admins and its
    /// name, and omits the description.
    #[test]
    fn test_get_contract_group_info_with_admins() {
        let info = ContractGroupInfo::V0(ContractGroupInfoV0 {
            owner: ContractGroupOwner::OwnerAndAdmins {
                owner: Identifier::new(OWNER_ID),
                admins: BTreeSet::from([Identifier::new(ADMIN_ID)]),
            },
            name: Some("connect".to_string()),
            description: None,
        });
        let handle = create_mock_sdk_handle_with_group(Some(info));
        let id = CString::new(base58(GROUP_ID)).unwrap();
        unsafe {
            let json = json_of(dash_sdk_contract_group_get_info(handle, id.as_ptr()));
            assert_eq!(
                json,
                serde_json::json!({
                    "ownerId": base58(OWNER_ID),
                    "adminIds": [base58(ADMIN_ID)],
                    "name": "connect",
                })
            );
        }
        destroy_mock_sdk_handle(handle);
    }

    /// A single-owner group reports an empty admin list.
    #[test]
    fn test_get_contract_group_info_single_owner() {
        let info = ContractGroupInfo::V0(ContractGroupInfoV0 {
            owner: ContractGroupOwner::SingleOwner(Identifier::new(OWNER_ID)),
            name: None,
            description: Some("apps of one developer".to_string()),
        });
        let handle = create_mock_sdk_handle_with_group(Some(info));
        let id = CString::new(base58(GROUP_ID)).unwrap();
        unsafe {
            let json = json_of(dash_sdk_contract_group_get_info(handle, id.as_ptr()));
            assert_eq!(
                json,
                serde_json::json!({
                    "ownerId": base58(OWNER_ID),
                    "adminIds": [],
                    "description": "apps of one developer",
                })
            );
        }
        destroy_mock_sdk_handle(handle);
    }

    /// An absent group is no data and no error, not a failure.
    #[test]
    fn test_get_contract_group_info_not_found() {
        let handle = create_mock_sdk_handle_with_group(None);
        let id = CString::new(base58(GROUP_ID)).unwrap();
        unsafe {
            let result = dash_sdk_contract_group_get_info(handle, id.as_ptr());
            assert!(result.error.is_null());
            assert!(result.data.is_null());
            assert_eq!(result.data_type, DashSDKResultDataType::NoData);
        }
        destroy_mock_sdk_handle(handle);
    }
}
