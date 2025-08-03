use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::Voter;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches voters for a contested resource identity
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `document_type_name` - Name of the document type
/// * `index_name` - Name of the index
/// * `index_values_json` - JSON array of hex-encoded index values
/// * `contestant_id` - Base58-encoded contestant identifier
/// * `count` - Maximum number of voters to return
/// * `order_ascending` - Whether to order results in ascending order
///
/// # Returns
/// * JSON array of voters or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_resource_get_voters_for_identity(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    contestant_id: *const c_char,
    count: u32,
    order_ascending: bool,
) -> DashSDKResult {
    match get_contested_resource_voters_for_identity(
        sdk_handle,
        contract_id,
        document_type_name,
        index_name,
        index_values_json,
        contestant_id,
        count,
        order_ascending,
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

fn get_contested_resource_voters_for_identity(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    contestant_id: *const c_char,
    count: u32,
    order_ascending: bool,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    if contract_id.is_null() {
        return Err("Contract ID is null".to_string());
    }

    if document_type_name.is_null() {
        return Err("Document type name is null".to_string());
    }

    if index_name.is_null() {
        return Err("Index name is null".to_string());
    }

    if index_values_json.is_null() {
        return Err("Index values JSON is null".to_string());
    }

    if contestant_id.is_null() {
        return Err("Contestant ID is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let contract_id_str = unsafe {
        CStr::from_ptr(contract_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contract ID: {}", e))?
    };
    let document_type_name_str = unsafe {
        CStr::from_ptr(document_type_name)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in document type name: {}", e))?
    };
    let index_name_str = unsafe {
        CStr::from_ptr(index_name)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in index name: {}", e))?
    };
    let index_values_str = unsafe {
        CStr::from_ptr(index_values_json)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in index values: {}", e))?
    };
    let contestant_id_str = unsafe {
        CStr::from_ptr(contestant_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in contestant ID: {}", e))?
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

        let contestant_id_bytes = bs58::decode(contestant_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode contestant ID: {}", e))?;

        let contestant_id: [u8; 32] = contestant_id_bytes
            .try_into()
            .map_err(|_| "Contestant ID must be exactly 32 bytes".to_string())?;

        let contract_id = dash_sdk::platform::Identifier::new(contract_id);
        let contestant_id = dash_sdk::platform::Identifier::new(contestant_id);

        // Parse index values
        let index_values_array: Vec<String> = serde_json::from_str(index_values_str)
            .map_err(|e| format!("Failed to parse index values JSON: {}", e))?;

        let index_values: Vec<Value> = index_values_array
            .into_iter()
            .map(|value_str| {
                // Check if the value is hex-encoded (all characters are valid hex)
                if value_str.chars().all(|c| c.is_ascii_hexdigit()) && value_str.len() % 2 == 0 {
                    // Try to decode as hex
                    match hex::decode(&value_str) {
                        Ok(bytes) => Ok(Value::Bytes(bytes)),
                        Err(_) => {
                            // If hex decode fails, treat as text
                            Ok(Value::Text(value_str))
                        }
                    }
                } else {
                    // Not hex, treat as text string
                    Ok(Value::Text(value_str))
                }
            })
            .collect::<Result<Vec<Value>, String>>()?;

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            index_values,
        };

        let query = ContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            offset: None,
            limit: Some(count as u16),
            start_at: None,
            order_ascending,
        };

        match Voter::fetch_many(&sdk, query).await {
            Ok(voters) => {
                if voters.0.is_empty() {
                    return Ok(None);
                }

                let voters_json: Vec<String> = voters
                    .0
                    .iter()
                    .map(|voter| {
                        format!(
                            r#"{{"voter_id":"{}"}}"#,
                            bs58::encode(voter.0.as_bytes()).into_string()
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", voters_json.join(","))))
            }
            Err(e) => Err(format!(
                "Failed to fetch contested resource voters for identity: {}",
                e
            )),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_contested_resource_voters_for_identity_null_handle() {
        unsafe {
            let result = dash_sdk_contested_resource_get_voters_for_identity(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                CString::new("contestant").unwrap().as_ptr(),
                10,
                true,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_contested_resource_voters_for_identity_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contested_resource_get_voters_for_identity(
                handle,
                std::ptr::null(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                CString::new("contestant").unwrap().as_ptr(),
                10,
                true,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
