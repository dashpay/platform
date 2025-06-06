use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchMany;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::{ContestedResource, ContestedResources};
use std::ffi::{c_char, CStr, CString};

/// Fetches contested resources
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `document_type_name` - Name of the document type
/// * `index_name` - Name of the index
/// * `start_index_values_json` - JSON array of hex-encoded start index values
/// * `end_index_values_json` - JSON array of hex-encoded end index values
/// * `count` - Maximum number of resources to return
/// * `order_ascending` - Whether to order results in ascending order
///
/// # Returns
/// * JSON array of contested resources or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_resource_get_resources(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    start_index_values_json: *const c_char,
    end_index_values_json: *const c_char,
    count: u32,
    order_ascending: bool,
) -> DashSDKResult {
    match get_contested_resources(
        sdk_handle,
        contract_id,
        document_type_name,
        index_name,
        start_index_values_json,
        end_index_values_json,
        count,
        order_ascending,
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

fn get_contested_resources(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    start_index_values_json: *const c_char,
    end_index_values_json: *const c_char,
    count: u32,
    order_ascending: bool,
) -> Result<Option<String>, String> {
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
    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        let contract_id_bytes = bs58::decode(contract_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode contract ID: {}", e))?;

        let contract_id: [u8; 32] = contract_id_bytes
            .try_into()
            .map_err(|_| "Contract ID must be exactly 32 bytes".to_string())?;

        let contract_id = dash_sdk::Identifier::new(contract_id);

        // Parse start index values
        let start_index_values = if start_index_values_json.is_null() {
            Vec::new()
        } else {
            let start_values_str = unsafe {
                CStr::from_ptr(start_index_values_json)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start index values: {}", e))?
            };
            let start_values_array: Vec<String> = serde_json::from_str(start_values_str)
                .map_err(|e| format!("Failed to parse start index values JSON: {}", e))?;
            
            start_values_array
                .into_iter()
                .map(|hex_str| {
                    hex::decode(&hex_str).map_err(|e| format!("Failed to decode start index value: {}", e))
                })
                .collect::<Result<Vec<Vec<u8>>, String>>()?
        };

        // Parse end index values
        let end_index_values = if end_index_values_json.is_null() {
            Vec::new()
        } else {
            let end_values_str = unsafe {
                CStr::from_ptr(end_index_values_json)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in end index values: {}", e))?
            };
            let end_values_array: Vec<String> = serde_json::from_str(end_values_str)
                .map_err(|e| format!("Failed to parse end index values JSON: {}", e))?;
            
            end_values_array
                .into_iter()
                .map(|hex_str| {
                    hex::decode(&hex_str).map_err(|e| format!("Failed to decode end index value: {}", e))
                })
                .collect::<Result<Vec<Vec<u8>>, String>>()?
        };

        let query = VotePollsByDocumentTypeQuery {
            contract_id,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            start_index_values,
            end_index_values,
            start_at_value_info: None,
            count: Some(count),
            order_ascending,
        };

        match ContestedResource::fetch_many(&sdk, query).await {
            Ok(resources) => {
                if resources.is_empty() {
                    return Ok(None);
                }

                let resources_json: Vec<String> = resources
                    .iter()
                    .map(|(id, resource)| {
                        format!(
                            r#"{{"id":"{}","contract_id":"{}","document_type_name":"{}","index_name":"{}","index_values":"{}"}}"#,
                            bs58::encode(id.as_bytes()).into_string(),
                            bs58::encode(resource.contract_id().as_bytes()).into_string(),
                            resource.document_type_name(),
                            resource.index_name(),
                            hex::encode(&resource.index_values())
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", resources_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch contested resources: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_contested_resources_null_handle() {
        unsafe {
            let result = dash_sdk_contested_resource_get_resources(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                10,
                true,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_contested_resources_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contested_resource_get_resources(
                handle,
                std::ptr::null(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                10,
                true,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
