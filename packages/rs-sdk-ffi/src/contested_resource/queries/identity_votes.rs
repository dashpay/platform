use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use dash_sdk::platform::FetchMany;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches contested resource identity votes
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `identity_id` - Base58-encoded identity identifier
/// * `limit` - Maximum number of votes to return (optional, 0 for no limit)
/// * `offset` - Number of votes to skip (optional, 0 for no offset)
/// * `order_ascending` - Whether to order results in ascending order
///
/// # Returns
/// * JSON array of votes or null if not found
/// * Error message if operation fails
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - `identity_id` must be a valid, non-null pointer to a NUL-terminated C string that remains valid during the call.
/// - `limit`, `offset`, and `order_ascending` are passed by value; no references are retained.
/// - On success, the returned `DashSDKResult` may contain a heap-allocated C string; the caller must free
///   it using the SDK's free routine. The result can also contain no data (null pointer).
/// - All pointers provided to this function must be readable and valid.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_contested_resource_get_identity_votes(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    limit: u32,
    offset: u32,
    order_ascending: bool,
) -> DashSDKResult {
    match get_contested_resource_identity_votes(
        sdk_handle,
        identity_id,
        limit,
        offset,
        order_ascending,
    ) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::NoData,
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
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

fn get_contested_resource_identity_votes(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    limit: u32,
    offset: u32,
    order_ascending: bool,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    if identity_id.is_null() {
        return Err("Identity ID is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let identity_id_str = unsafe {
        CStr::from_ptr(identity_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in identity ID: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let identity_id_bytes = bs58::decode(identity_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode identity ID: {}", e))?;

        let identity_id: [u8; 32] = identity_id_bytes
            .try_into()
            .map_err(|_| "Identity ID must be exactly 32 bytes".to_string())?;

        let identity_id = dash_sdk::platform::Identifier::new(identity_id);

        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            start_at: None,
            limit: if limit > 0 { Some(limit as u16) } else { None },
            offset: if offset > 0 { Some(offset as u16) } else { None },
            order_ascending,
        };

        match ResourceVote::fetch_many(&sdk, query).await {
            Ok(votes_map) => {
                if votes_map.is_empty() {
                    return Ok(None);
                }

                let votes_json: Vec<String> = votes_map
                    .iter()
                    .filter_map(|(vote_poll_id, vote_option)| {
                        vote_option.as_ref().map(|resource_vote| {
                            let vote_type = match &resource_vote.resource_vote_choice() {
                                    dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity(id) => {
                                        format!(r#"{{"type":"towards_identity","identity_id":"{}"}}"#, 
                                            bs58::encode(id.as_bytes()).into_string())
                                    }
                                    dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::Abstain => {
                                        r#"{"type":"abstain"}"#.to_string()
                                    }
                                    dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::Lock => {
                                        r#"{"type":"lock"}"#.to_string()
                                    }
                            };

                        format!(
                            r#"{{"vote_poll_id":"{}","resource_vote_choice":{}}}"#,
                            bs58::encode(vote_poll_id.as_bytes()).into_string(),
                            vote_type
                        )
                        })
                    })
                    .collect();

                Ok(Some(format!("[{}]", votes_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch contested resource identity votes: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_contested_resource_identity_votes_null_handle() {
        unsafe {
            let result = dash_sdk_contested_resource_get_identity_votes(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
                10,
                0,
                true,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_contested_resource_identity_votes_null_identity_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_contested_resource_get_identity_votes(
                handle,
                std::ptr::null(),
                10,
                0,
                true,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
