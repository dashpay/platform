use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::get_token_pre_programmed_distributions_request_v0::StartAtInfo;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::GetTokenPreProgrammedDistributionsRequestV0;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::{get_token_pre_programmed_distributions_response_v0, GetTokenPreProgrammedDistributionsResponseV0};
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::get_token_pre_programmed_distributions_response_v0::{TokenDistributions, TokenDistributionEntry, TokenTimedDistributionEntry};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_token_pre_programmed_distributions_v0(
        &self,
        GetTokenPreProgrammedDistributionsRequestV0 {
            token_id,
            start_at_info,
            limit,
            prove,
        }: GetTokenPreProgrammedDistributionsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenPreProgrammedDistributionsResponseV0>, Error> {
        let config = &self.config.drive;
        let token_id: Identifier =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = limit
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.default_query_limit
                {
                    None
                } else {
                    Some(limit_value as u16)
                }
            })
            .ok_or(drive::error::Error::Query(QuerySyntaxError::InvalidLimit(
                format!("limit greater than max limit {}", config.max_query_limit),
            )))?;

        let start_at = match start_at_info {
            None => None,
            Some(StartAtInfo {
                start_time_ms,
                start_recipient,
                start_recipient_included,
            }) => {
                let start_at_recipient = match start_recipient {
                    None => None,
                    Some(identifier) => {
                        let recipient_id: Identifier = check_validation_result_with_data!(
                            identifier.try_into().map_err(|_| {
                                QueryError::InvalidArgument(
                                    "start_recipient must be a valid identifier (32 bytes long)"
                                        .to_string(),
                                )
                            })
                        );
                        Some((recipient_id, start_recipient_included.unwrap_or(true)))
                    }
                };

                Some(QueryPreProgrammedDistributionStartAt {
                    start_at_time: start_time_ms,
                    start_at_recipient,
                })
            }
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_token_pre_programmed_distributions(
                    token_id.into_buffer(),
                    start_at,
                    Some(limit),
                    None,
                    platform_version,
                ));

            GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(
                    get_token_pre_programmed_distributions_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                            .map(|(_, proof)| proof)?,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let token_distributions = self
                .drive
                .fetch_token_pre_programmed_distributions(
                    token_id.into_buffer(),
                    start_at,
                    Some(limit),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(timestamp, distributions_for_time)| {
                    let distributions = distributions_for_time
                        .into_iter()
                        .map(|(recipient, amount)| TokenDistributionEntry {
                            recipient_id: recipient.to_vec(),
                            amount,
                        })
                        .collect();
                    TokenTimedDistributionEntry {
                        timestamp,
                        distributions,
                    }
                })
                .collect();

            GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(
                    get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                        TokenDistributions {
                            token_distributions,
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
    use crate::query::tests::setup_platform_with_token_state;
    use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::get_token_pre_programmed_distributions_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![0; 8],
            start_at_info: None,
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![0; 32],
            start_at_info: None,
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                    dists,
                ),
            ) => {
                assert!(dists.token_distributions.is_empty());
            }
            _ => panic!("expected TokenDistributions result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![0; 32],
            start_at_info: None,
            limit: None,
            prove: true,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(get_token_pre_programmed_distributions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_distribution_data() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 2 has pre-programmed distributions at timestamps 1000, 5000, 10000
        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: None,
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                    dists,
                ),
            ) => {
                assert_eq!(dists.token_distributions.len(), 3);
                // Verify timestamps
                let timestamps: Vec<u64> = dists
                    .token_distributions
                    .iter()
                    .map(|d| d.timestamp)
                    .collect();
                assert!(timestamps.contains(&1000));
                assert!(timestamps.contains(&5000));
                assert!(timestamps.contains(&10000));
            }
            _ => panic!("expected TokenDistributions result"),
        }
    }

    #[test]
    fn test_query_with_limit() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: None,
            limit: Some(2),
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                    dists,
                ),
            ) => {
                // With limit=2, we should get fewer distributions than the full set (3)
                assert!(dists.token_distributions.len() < 3);
                assert!(!dists.token_distributions.is_empty());
            }
            _ => panic!("expected TokenDistributions result"),
        }
    }

    #[test]
    fn test_query_with_start_at() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 5000,
                start_recipient: None,
                start_recipient_included: None,
            }),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                    dists,
                ),
            ) => {
                // Should return distributions starting from 5000 onwards
                assert!(!dists.token_distributions.is_empty());
                for d in &dists.token_distributions {
                    assert!(d.timestamp >= 5000);
                }
            }
            _ => panic!("expected TokenDistributions result"),
        }
    }

    #[test]
    fn test_query_with_invalid_start_recipient() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 1000,
                start_recipient: Some(vec![0; 8]), // invalid
                start_recipient_included: None,
            }),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start_recipient")
        ));
    }

    #[test]
    fn test_query_no_distributions_for_token() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 0 has no pre-programmed distributions
        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[0].to_vec(),
            start_at_info: None,
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                    dists,
                ),
            ) => {
                assert!(dists.token_distributions.is_empty());
            }
            _ => panic!("expected TokenDistributions result"),
        }
    }

    #[test]
    fn test_query_with_data_proof() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: None,
            limit: None,
            prove: true,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(get_token_pre_programmed_distributions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_limit_zero_is_rejected_as_error() {
        // limit == 0 routes through the `.ok_or(...)?` path, which propagates
        // a Drive(Query(InvalidLimit(...))) error via `Err(...)`, not as a
        // validation error inside the response.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![0; 32],
            start_at_info: None,
            limit: Some(0),
            prove: false,
        };

        let err = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect_err("limit=0 should propagate an Err");

        // Accept any Drive/Query related error; we just want to verify the
        // `.ok_or(...)?` path is exercised.
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("InvalidLimit") || msg.contains("limit"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_query_limit_above_max_is_rejected_as_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![0; 32],
            start_at_info: None,
            limit: Some((u16::MAX as u32) + 1),
            prove: false,
        };

        let err = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect_err("oversized limit should propagate an Err");

        let msg = format!("{:?}", err);
        assert!(
            msg.contains("InvalidLimit") || msg.contains("limit"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_invalid_token_id_zero_length() {
        // Completely empty token_id bytes should fail the identifier check.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![],
            start_at_info: None,
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_with_start_at_recipient_included_false() {
        // start_recipient supplied + start_recipient_included = Some(false).
        // Exercises the "Some(false)" branch of the unwrap_or(true).
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 1000,
                start_recipient: Some(identity_ids[0].to_vec()),
                start_recipient_included: Some(false),
            }),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        assert!(matches!(
            data.result,
            Some(get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(_))
        ));
    }

    #[test]
    fn test_query_with_start_at_recipient_included_true() {
        // Explicit Some(true) — the default branch is unwrap_or(true) so this
        // is the same behavior as passing None; but we want to cover the
        // explicit wrapping.
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 1000,
                start_recipient: Some(identity_ids[0].to_vec()),
                start_recipient_included: Some(true),
            }),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn test_query_with_start_at_recipient_included_none() {
        // start_recipient provided but included=None (→ defaults to true).
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 1000,
                start_recipient: Some(identity_ids[0].to_vec()),
                start_recipient_included: None,
            }),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn test_query_with_proof_and_start_at() {
        // Exercises the prove path when start_at is supplied.
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 5000,
                start_recipient: None,
                start_recipient_included: None,
            }),
            limit: Some(5),
            prove: true,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(matches!(
            result.data,
            Some(GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(get_token_pre_programmed_distributions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_start_at_future_returns_empty() {
        // Start-time far past any existing distribution → empty result, but
        // still success.
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: u64::MAX,
                start_recipient: None,
                start_recipient_included: None,
            }),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                    dists,
                ),
            ) => {
                assert!(dists.token_distributions.is_empty());
            }
            _ => panic!("expected TokenDistributions result"),
        }
    }

    #[test]
    fn test_query_with_proof_and_invalid_token_id() {
        // Invalid token id short-circuits before the prove branch.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![1; 31],
            start_at_info: None,
            limit: None,
            prove: true,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_with_invalid_start_recipient_with_proof() {
        // Invalid start_recipient also fails in the prove path.
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: token_ids[2].to_vec(),
            start_at_info: Some(StartAtInfo {
                start_time_ms: 1000,
                start_recipient: Some(vec![0; 7]),
                start_recipient_included: None,
            }),
            limit: None,
            prove: true,
        };

        let result = platform
            .query_token_pre_programmed_distributions_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start_recipient")
        ));
    }
}
