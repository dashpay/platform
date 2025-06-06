use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchMany;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
use drive_proof_verifier::types::Contenders;
use std::ffi::{c_char, CStr, CString};

/// Fetches contested resource vote state
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `contract_id` - Base58-encoded contract identifier
/// * `document_type_name` - Name of the document type
/// * `index_name` - Name of the index
/// * `index_values_json` - JSON array of hex-encoded index values
/// * `result_type` - Result type (0=DOCUMENTS, 1=VOTE_TALLY, 2=DOCUMENTS_AND_VOTE_TALLY)
/// * `allow_include_locked_and_abstaining_vote_tally` - Whether to include locked and abstaining votes
/// * `count` - Maximum number of results to return
///
/// # Returns
/// * JSON array of contenders or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_resource_get_vote_state(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    result_type: u8,
    allow_include_locked_and_abstaining_vote_tally: bool,
    count: u32,
) -> DashSDKResult {
    match get_contested_resource_vote_state(
        sdk_handle,
        contract_id,
        document_type_name,
        index_name,
        index_values_json,
        result_type,
        allow_include_locked_and_abstaining_vote_tally,
        count,
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

fn get_contested_resource_vote_state(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    document_type_name: *const c_char,
    index_name: *const c_char,
    index_values_json: *const c_char,
    result_type: u8,
    allow_include_locked_and_abstaining_vote_tally: bool,
    count: u32,
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
    let index_values_str = unsafe {
        CStr::from_ptr(index_values_json)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in index values: {}", e))?
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

        // Parse index values
        let index_values_array: Vec<String> = serde_json::from_str(index_values_str)
            .map_err(|e| format!("Failed to parse index values JSON: {}", e))?;

        let index_values: Vec<Vec<u8>> = index_values_array
            .into_iter()
            .map(|hex_str| {
                hex::decode(&hex_str).map_err(|e| format!("Failed to decode index value: {}", e))
            })
            .collect::<Result<Vec<Vec<u8>>, String>>()?;

        let result_type = match result_type {
            0 => drive::query::vote_poll_vote_state_query::VotePollVoteStateResultType::DocumentsOnly,
            1 => drive::query::vote_poll_vote_state_query::VotePollVoteStateResultType::VoteTallyOnly,
            2 => drive::query::vote_poll_vote_state_query::VotePollVoteStateResultType::DocumentsAndVoteTally,
            _ => return Err("Invalid result type".to_string()),
        };

        let query = ContestedDocumentVotePollDriveQuery {
            contract_id,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            index_values,
            result_type,
            start_at_identifier_info: None,
            count: Some(count),
            allow_include_locked_and_abstaining_vote_tally,
            offset: None,
        };

        match ContenderWithSerializedDocument::fetch_many(&sdk, query).await {
            Ok(contenders) => {
                if contenders.is_empty() {
                    return Ok(None);
                }

                let contenders_json: Vec<String> = contenders
                    .iter()
                    .map(|(id, contender)| {
                        let document_json = if let Some(ref document) = contender.document {
                            format!(r#""document":{}"#, serde_json::to_string(document).unwrap_or_else(|_| "null".to_string()))
                        } else {
                            r#""document":null"#.to_string()
                        };

                        let vote_tally_json = if let Some(ref vote_tally) = contender.vote_tally {
                            format!(r#""vote_tally":{{"abstain_vote_tally":{},"lock_vote_tally":{}}}"#, 
                                vote_tally.abstain_vote_tally, vote_tally.lock_vote_tally)
                        } else {
                            r#""vote_tally":null"#.to_string()
                        };

                        format!(
                            r#"{{"id":"{}",{},{}}}"#,
                            bs58::encode(id.as_bytes()).into_string(),
                            document_json,
                            vote_tally_json
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", contenders_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch contested resource vote state: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_contested_resource_vote_state_null_handle() {
        unsafe {
            let result = dash_sdk_contested_resource_get_vote_state(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                0,
                false,
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_contested_resource_vote_state_null_contract_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contested_resource_get_vote_state(
                handle,
                std::ptr::null(),
                CString::new("type").unwrap().as_ptr(),
                CString::new("index").unwrap().as_ptr(),
                CString::new(r#"["00"]"#).unwrap().as_ptr(),
                0,
                false,
                10,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
