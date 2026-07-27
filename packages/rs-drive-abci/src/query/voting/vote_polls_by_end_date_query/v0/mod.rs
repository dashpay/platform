use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::GetVotePollsByEndDateRequestV0;
use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::{get_vote_polls_by_end_date_response_v0, GetVotePollsByEndDateResponseV0};
use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamp;
use dpp::check_validation_result_with_data;
use dpp::version::PlatformVersion;
use dpp::validation::ValidationResult;

use crate::query::response_metadata::CheckpointUsed;
use drive::error::query::QuerySyntaxError;
use drive::query::VotePollsByEndDateDriveQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_vote_polls_by_end_date_query_v0(
        &self,
        GetVotePollsByEndDateRequestV0 {
            start_time_info,
            end_time_info,
            limit,
            offset,
            ascending,
            prove,
        }: GetVotePollsByEndDateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetVotePollsByEndDateResponseV0>, Error> {
        let config = &self.config.drive;

        let start_time = start_time_info.map(|start_time_info| {
            (
                start_time_info.start_time_ms,
                start_time_info.start_time_included,
            )
        });

        let end_time = end_time_info
            .map(|end_time_info| (end_time_info.end_time_ms, end_time_info.end_time_included));

        if let (
            Some((start_time_ms, start_time_included)),
            Some((end_time_ms, end_time_included)),
        ) = (start_time, end_time)
        {
            if start_time_ms > end_time_ms {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!(
                        "start time {} must be before end time {}",
                        start_time_ms, end_time_ms
                    )),
                ));
            }
            if start_time_ms == end_time_ms && (!start_time_included || !end_time_included) {
                return Ok(QueryValidationResult::new_with_error(QueryError::InvalidArgument("if start time is equal to end time the start and end times must be included".to_string())));
            }
        };

        let limit = check_validation_result_with_data!(limit.map_or(
            Ok(config.default_query_limit),
            |limit| {
                let limit = u16::try_from(limit)
                    .map_err(|_| QueryError::InvalidArgument("limit out of bounds".to_string()))?;
                if limit == 0 || limit > config.default_query_limit {
                    Err(QueryError::InvalidArgument(format!(
                        "limit {} out of bounds of [1, {}]",
                        limit, config.default_query_limit
                    )))
                } else {
                    Ok(limit)
                }
            }
        ));

        let offset = check_validation_result_with_data!(offset
            .map(|offset| {
                u16::try_from(offset)
                    .map_err(|_| QueryError::InvalidArgument("offset out of bounds".to_string()))
            })
            .transpose());

        let prove_offset = offset.unwrap_or(0);
        if prove && prove_offset != 0 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::RequestingProofWithOffset(format!(
                    "requesting proved contested vote polls by end date with an offset {}",
                    prove_offset
                )),
            )));
        }

        let query = VotePollsByEndDateDriveQuery {
            start_time,
            limit: Some(limit),
            offset,
            order_ascending: ascending,
            end_time,
        };

        let response = if prove {
            let proof = match query.execute_with_proof(&self.drive, None, None, platform_version) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetVotePollsByEndDateResponseV0 {
                result: Some(get_vote_polls_by_end_date_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let results = match query.execute_no_proof_keep_serialized(
                &self.drive,
                None,
                &mut vec![],
                platform_version,
            ) {
                Ok(result) => result,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            let (vote_polls_by_timestamps, counts): (
                Vec<SerializedVotePollsByTimestamp>,
                Vec<usize>,
            ) = if query.order_ascending {
                results
                    .into_iter()
                    .map(|(timestamp, contested_document_resource_vote_polls)| {
                        let len = contested_document_resource_vote_polls.len();
                        (
                            SerializedVotePollsByTimestamp {
                                timestamp,
                                serialized_vote_polls: contested_document_resource_vote_polls,
                            },
                            len,
                        )
                    })
                    .unzip()
            } else {
                results
                    .into_iter()
                    .rev()
                    .map(|(timestamp, contested_document_resource_vote_polls)| {
                        let len = contested_document_resource_vote_polls.len();
                        (
                            SerializedVotePollsByTimestamp {
                                timestamp,
                                serialized_vote_polls: contested_document_resource_vote_polls,
                            },
                            len,
                        )
                    })
                    .unzip()
            };

            let count: usize = counts.into_iter().sum();

            let finished_results = if count as u16 == limit {
                let last = vote_polls_by_timestamps
                    .last()
                    .expect("there should be a last one if count exists");
                let next_query = VotePollsByEndDateDriveQuery {
                    start_time: Some((last.timestamp, false)),
                    limit: Some(1),
                    offset: None,
                    order_ascending: ascending,
                    end_time,
                };

                let next_query_results = match next_query.execute_no_proof_keep_serialized(
                    &self.drive,
                    None,
                    &mut vec![],
                    platform_version,
                ) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };
                next_query_results.is_empty()
            } else {
                true
            };

            GetVotePollsByEndDateResponseV0 {
                result: Some(
                    get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                        get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                            vote_polls_by_timestamps,
                            finished_results,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::get_vote_polls_by_end_date_request_v0::{
        EndAtTimeInfo, StartAtTimeInfo,
    };
    use dpp::dashcore::Network;

    #[test]
    fn test_query_vote_polls_by_end_date_start_after_end() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: Some(StartAtTimeInfo {
                start_time_ms: 1000,
                start_time_included: true,
            }),
            end_time_info: Some(EndAtTimeInfo {
                end_time_ms: 500,
                end_time_included: true,
            }),
            limit: None,
            offset: None,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("must be before end time")
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_equal_times_without_both_included() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: Some(StartAtTimeInfo {
                start_time_ms: 1000,
                start_time_included: true,
            }),
            end_time_info: Some(EndAtTimeInfo {
                end_time_ms: 1000,
                end_time_included: false,
            }),
            limit: None,
            offset: None,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start and end times must be included")
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_limit_zero() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: Some(0),
            offset: None,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("out of bounds of")
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_offset_out_of_bounds() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: None,
            offset: Some((u16::MAX as u32) + 1),
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("offset out of bounds")
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_prove_with_offset() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: None,
            offset: Some(5),
            ascending: true,
            prove: true,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                QuerySyntaxError::RequestingProofWithOffset(_)
            )]
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: None,
            offset: None,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetVotePollsByEndDateResponseV0 {
                result: Some(
                    get_vote_polls_by_end_date_response_v0::Result::VotePollsByTimestamps(
                        get_vote_polls_by_end_date_response_v0::SerializedVotePollsByTimestamps {
                            vote_polls_by_timestamps,
                            finished_results,
                        },
                    ),
                ),
                metadata: Some(_),
            }) if vote_polls_by_timestamps.is_empty() && finished_results
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_limit_out_of_u16_range() {
        // limit > u16::MAX triggers the u16::try_from failure path.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: Some((u16::MAX as u32) + 1),
            offset: None,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("limit out of bounds")
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_limit_above_default_but_within_u16() {
        // A limit that fits in u16 but exceeds default_query_limit should be
        // rejected with the "out of bounds of [1, <default>]" message.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let over_default = (platform.config.drive.default_query_limit as u32).saturating_add(1);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: Some(over_default),
            offset: None,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("out of bounds of")
        ));
    }

    #[test]
    fn test_query_vote_polls_by_end_date_empty_prove() {
        // Exercise the prove=true branch with offset=None so the
        // RequestingProofWithOffset guard is skipped.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetVotePollsByEndDateRequestV0 {
            start_time_info: None,
            end_time_info: None,
            limit: None,
            offset: None,
            ascending: true,
            prove: true,
        };

        let result = platform
            .query_vote_polls_by_end_date_query_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetVotePollsByEndDateResponseV0 {
                result: Some(get_vote_polls_by_end_date_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
