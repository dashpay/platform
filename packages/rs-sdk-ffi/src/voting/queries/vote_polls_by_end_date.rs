use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::serialization::JsonConvertible;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::drive::query::VotePollsByEndDateDriveQuery;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::VotePollsGroupedByTimestamp;
use std::ffi::{c_void, CString};

/// Fetches vote polls by end date
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_time_ms` - Start time in milliseconds (optional, 0 for no start time)
/// * `start_time_included` - Whether to include the start time
/// * `end_time_ms` - End time in milliseconds (optional, 0 for no end time)
/// * `end_time_included` - Whether to include the end time
/// * `limit` - Maximum number of results to return (optional, 0 for no limit)
/// * `offset` - Number of results to skip (optional, 0 for no offset)
/// * `ascending` - Whether to order results in ascending order
///
/// # Returns
/// * JSON array of the vote polls grouped by end date, or null when there is none:
///   `[{"timestamp": <end date ms>, "vote_polls": [<vote poll>, ...]}, ...]`. Each vote poll
///   is its canonical JSON, tagged with `$type`:
///   - a contested resource poll:
///     `{"$type": "contestedDocumentResourceVotePoll", "contractId": "<base58>",
///     "documentTypeName": "domain", "indexName": "parentNameAndLabel",
///     "indexValues": ["dash", "alice"]}`;
///   - a yes/no poll (protocol version 15), such as a decision proposal's:
///     `{"$type": "yesNoVotePoll", "purpose": {"$type": "decisionProposal",
///     "documentId": "<base58 proposal id>"}, "supermajorityNumerator": 60,
///     "supermajorityDenominator": 100, "minimumVotingPower": {"$type": "fractionOfTotal",
///     "numerator": 10, "denominator": 100, "rounding": "up"}}`.
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_voting_get_vote_polls_by_end_date(
    sdk_handle: *const SDKHandle,
    start_time_ms: u64,
    start_time_included: bool,
    end_time_ms: u64,
    end_time_included: bool,
    limit: u32,
    offset: u32,
    ascending: bool,
) -> DashSDKResult {
    match get_vote_polls_by_end_date(
        sdk_handle,
        start_time_ms,
        start_time_included,
        end_time_ms,
        end_time_included,
        limit,
        offset,
        ascending,
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

#[allow(clippy::too_many_arguments)]
fn get_vote_polls_by_end_date(
    sdk_handle: *const SDKHandle,
    start_time_ms: u64,
    start_time_included: bool,
    end_time_ms: u64,
    end_time_included: bool,
    limit: u32,
    offset: u32,
    ascending: bool,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let start_time_info = if start_time_ms > 0 {
            Some((start_time_ms, start_time_included))
        } else {
            None
        };

        let end_time_info = if end_time_ms > 0 {
            Some((end_time_ms, end_time_included))
        } else {
            None
        };

        let query = VotePollsByEndDateDriveQuery {
            start_time: start_time_info,
            end_time: end_time_info,
            limit: if limit > 0 { Some(limit as u16) } else { None },
            offset: if offset > 0 {
                Some(offset as u16)
            } else {
                None
            },
            order_ascending: ascending,
        };

        match VotePoll::fetch_many(&sdk, query).await {
            Ok(vote_polls_grouped) => vote_polls_by_end_date_json(&vote_polls_grouped),
            Err(e) => Err(format!("Failed to fetch vote polls by end date: {}", e)),
        }
    })
}

/// The JSON `dash_sdk_voting_get_vote_polls_by_end_date` returns for `grouped`, `None` when
/// it holds no end date. Each poll is written whole, as its canonical JSON, so a yes/no poll
/// keeps its purpose (a decision proposal's names the proposal document) and its rule.
fn vote_polls_by_end_date_json(
    grouped: &VotePollsGroupedByTimestamp,
) -> Result<Option<String>, String> {
    if grouped.0.is_empty() {
        return Ok(None);
    }
    let groups = grouped
        .0
        .iter()
        .map(|(timestamp, vote_polls)| {
            let vote_polls = vote_polls
                .iter()
                .map(|vote_poll| {
                    vote_poll
                        .to_json()
                        .map_err(|e| format!("Failed to write vote poll {vote_poll} as JSON: {e}"))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(serde_json::json!({
                "timestamp": timestamp,
                "vote_polls": vote_polls,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_string(&groups)
        .map(Some)
        .map_err(|e| format!("Failed to serialize vote polls: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;
    use dash_sdk::dpp::platform_value::string_encoding::Encoding;
    use dash_sdk::dpp::platform_value::{Identifier, Value};
    use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dash_sdk::dpp::voting::vote_polls::yes_no_vote_poll::{
        VotingPowerRounding, YesNoMinimumVotingPower, YesNoVotePoll, YesNoVotePollPurpose,
    };

    #[test]
    fn test_get_vote_polls_by_end_date_null_handle() {
        unsafe {
            let result = dash_sdk_voting_get_vote_polls_by_end_date(
                std::ptr::null(),
                0,
                false,
                0,
                false,
                10,
                0,
                true,
            );
            assert!(!result.error.is_null());
        }
    }

    /// A yes/no poll keeps its purpose and rule, and a contested poll its index values, where
    /// each used to be written as its end date alone.
    #[test]
    fn should_write_each_vote_poll_whole_with_its_kind() {
        let proposal_id = Identifier::new([7; 32]);
        let contested =
            VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
                contract_id: Identifier::new([1; 32]),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text("alice".to_string()),
                ],
            });
        let yes_no = VotePoll::YesNoVotePoll(YesNoVotePoll {
            purpose: YesNoVotePollPurpose::DecisionProposal {
                document_id: proposal_id,
            },
            supermajority_numerator: 60,
            supermajority_denominator: 100,
            minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                numerator: 10,
                denominator: 100,
                rounding: VotingPowerRounding::Up,
            },
        });
        let grouped =
            VotePollsGroupedByTimestamp(vec![(1_000, vec![contested]), (2_000, vec![yes_no])]);

        let json: serde_json::Value = serde_json::from_str(
            &vote_polls_by_end_date_json(&grouped)
                .expect("serializes")
                .expect("not empty"),
        )
        .expect("JSON");
        assert_eq!(
            json,
            serde_json::json!([
                {
                    "timestamp": 1_000,
                    "vote_polls": [{
                        "$type": "contestedDocumentResourceVotePoll",
                        "contractId": Identifier::new([1; 32]).to_string(Encoding::Base58),
                        "documentTypeName": "domain",
                        "indexName": "parentNameAndLabel",
                        "indexValues": ["dash", "alice"],
                    }],
                },
                {
                    "timestamp": 2_000,
                    "vote_polls": [{
                        "$type": "yesNoVotePoll",
                        "purpose": {
                            "$type": "decisionProposal",
                            "documentId": proposal_id.to_string(Encoding::Base58),
                        },
                        "supermajorityNumerator": 60,
                        "supermajorityDenominator": 100,
                        "minimumVotingPower": {
                            "$type": "fractionOfTotal",
                            "numerator": 10,
                            "denominator": 100,
                            "rounding": "up",
                        },
                    }],
                },
            ])
        );
        assert_eq!(
            vote_polls_by_end_date_json(&VotePollsGroupedByTimestamp(vec![])),
            Ok(None)
        );
    }

    #[test]
    fn test_get_vote_polls_by_end_date() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let _result =
                dash_sdk_voting_get_vote_polls_by_end_date(handle, 0, false, 0, false, 10, 0, true);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
