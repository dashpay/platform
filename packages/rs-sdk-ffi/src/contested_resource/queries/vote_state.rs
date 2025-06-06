use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::Contenders;
use std::ffi::{c_char, c_void, CStr, CString};

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

        // Parse index values
        let index_values_array: Vec<String> = serde_json::from_str(index_values_str)
            .map_err(|e| format!("Failed to parse index values JSON: {}", e))?;

        let index_values: Vec<Value> = index_values_array
            .into_iter()
            .map(|hex_str| {
                let bytes = hex::decode(&hex_str).map_err(|e| format!("Failed to decode index value: {}", e))?;
                Ok(Value::Bytes(bytes))
            })
            .collect::<Result<Vec<Value>, String>>()?;

        let result_type = match result_type {
            0 => dash_sdk::drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::Documents,
            1 => dash_sdk::drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            2 => dash_sdk::drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            _ => return Err("Invalid result type".to_string()),
        };

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name: document_type_name_str.to_string(),
            index_name: index_name_str.to_string(),
            index_values,
        };
        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            limit: Some(count as u16),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally,
            offset: None,
        };

        match ContenderWithSerializedDocument::fetch_many(&sdk, query).await {
            Ok(contenders) => {
                let contenders: Contenders = contenders;
                if contenders.contenders.is_empty() {
                    return Ok(None);
                }

                let mut result_json_parts = Vec::new();
                // Add vote tally info if available
                if result_type.has_vote_tally() {
                    result_json_parts.push(format!(
                        r#""abstain_vote_tally":{},"lock_vote_tally":{}"#,
                        contenders.abstain_vote_tally.unwrap_or(0),
                        contenders.lock_vote_tally.unwrap_or(0)
                    ));
                }
                // Add winner info if available
                if let Some((winner_info, block_info)) = contenders.winner {
                    let winner_json = match winner_info {
                        ContestedDocumentVotePollWinnerInfo::NoWinner => {
                            r#""winner_info":"NoWinner""#.to_string()
                        }
                        ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier) => {
                            format!(r#""winner_info":{{"type":"WonByIdentity","identity_id":"{}"}}"#, bs58::encode(identifier.as_bytes()).into_string())
                        }
                        ContestedDocumentVotePollWinnerInfo::Locked => {
                            r#""winner_info":"Locked""#.to_string()
                        }
                    };
                    result_json_parts.push(format!(
                        r#"{},
                        "block_info":{{"height":{},"core_height":{},"timestamp":{}}}"#,
                        winner_json,
                        block_info.height,
                        block_info.core_height,
                        block_info.time_ms
                    ));
                }
                // Add contenders
                if result_type.has_documents() {
                    let contenders_json: Vec<String> = contenders.contenders
                        .iter()
                        .map(|(id, contender)| {
                            let document_json = if let Some(ref document) = contender.serialized_document() {
                                format!(r#""document":"{}""#, 
                                    hex::encode(document))
                            } else {
                                r#""document":null"#.to_string()
                            };

                            let vote_count = contender.vote_tally().unwrap_or(0);

                            format!(
                                r#"{{"identity_id":"{}","vote_count":{},{}}}"#,
                                bs58::encode(id.as_bytes()).into_string(),
                                vote_count,
                                document_json
                            )
                        })
                        .collect();
                    result_json_parts.push(format!(r#""contenders":[{}]"#, contenders_json.join(",")));
                }
                Ok(Some(format!("{{{}}}", result_json_parts.join(","))))
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
