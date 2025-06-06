use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKResult, FFIError};
use dash_sdk::platform::FetchMany;
use dashcore_rpc::dashcore::ProTxHash;
use drive_proof_verifier::types::{MasternodeProtocolVote, MasternodeProtocolVotes};
use std::ffi::{c_char, CStr, CString};

/// Fetches protocol version upgrade vote status
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_pro_tx_hash` - Starting masternode pro_tx_hash (hex-encoded, optional)
/// * `count` - Number of vote entries to retrieve
///
/// # Returns
/// * JSON array of masternode protocol version votes or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_protocol_version_get_upgrade_vote_status(
    sdk_handle: *const SDKHandle,
    start_pro_tx_hash: *const c_char,
    count: u32,
) -> DashSDKResult {
    match get_protocol_version_upgrade_vote_status(sdk_handle, start_pro_tx_hash, count) {
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

fn get_protocol_version_upgrade_vote_status(
    sdk_handle: *const SDKHandle,
    start_pro_tx_hash: *const c_char,
    count: u32,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let sdk = unsafe { &*sdk_handle }.sdk.clone();

    rt.block_on(async move {
        let start_hash = if start_pro_tx_hash.is_null() {
            None
        } else {
            let start_hash_str = unsafe {
                CStr::from_ptr(start_pro_tx_hash)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start pro_tx_hash: {}", e))?
            };
            let bytes = hex::decode(start_hash_str)
                .map_err(|e| format!("Failed to decode start pro_tx_hash: {}", e))?;
            let hash_bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| "start_pro_tx_hash must be exactly 32 bytes".to_string())?;
            Some(ProTxHash::from(hash_bytes))
        };

        let query = dash_sdk::platform::LimitQuery {
            query: ProtocolVersionUpgradeVoteStatusQuery {
                start_pro_tx_hash: start_hash,
            },
            limit: Some(count),
            start_info: None,
        };

        match MasternodeProtocolVote::fetch_many(&sdk, query).await {
            Ok(votes) => {
                if votes.is_empty() {
                    return Ok(None);
                }

                let votes_json: Vec<String> = votes
                    .iter()
                    .map(|(pro_tx_hash, vote)| {
                        format!(
                            r#"{{"pro_tx_hash":"{}","version":{}}}"#,
                            hex::encode(vote.pro_tx_hash.to_byte_array()),
                            vote.voted_version
                        )
                    })
                    .collect();

                Ok(Some(format!("[{}]", votes_json.join(","))))
            }
            Err(e) => Err(format!(
                "Failed to fetch protocol version upgrade vote status: {}",
                e
            )),
        }
    })
}

// Helper struct for the query
#[derive(Debug, Clone)]
struct ProtocolVersionUpgradeVoteStatusQuery {
    pub start_pro_tx_hash: Option<ProTxHash>,
}

impl dash_sdk::platform::Query<dapi_grpc::platform::v0::GetProtocolVersionUpgradeVoteStatusRequest>
    for ProtocolVersionUpgradeVoteStatusQuery
{
    fn query(
        self,
        prove: bool,
    ) -> Result<dapi_grpc::platform::v0::GetProtocolVersionUpgradeVoteStatusRequest, dash_sdk::Error>
    {
        use dapi_grpc::platform::v0::{
            get_protocol_version_upgrade_vote_status_request::{
                GetProtocolVersionUpgradeVoteStatusRequestV0, Version,
            },
            GetProtocolVersionUpgradeVoteStatusRequest,
        };

        let request = GetProtocolVersionUpgradeVoteStatusRequest {
            version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusRequestV0 {
                start_pro_tx_hash: self
                    .start_pro_tx_hash
                    .map(|hash| hash.to_byte_array().to_vec())
                    .unwrap_or_default(),
                count: 0, // Count is handled by LimitQuery wrapper
                prove,
            })),
        };

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_protocol_version_upgrade_vote_status_null_handle() {
        unsafe {
            let result = dash_sdk_protocol_version_get_upgrade_vote_status(
                std::ptr::null(),
                std::ptr::null(),
                10,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_protocol_version_upgrade_vote_status() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result =
                dash_sdk_protocol_version_get_upgrade_vote_status(handle, std::ptr::null(), 10);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
