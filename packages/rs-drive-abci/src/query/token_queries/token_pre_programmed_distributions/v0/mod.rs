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

        // A refused limit is a request error: returned through `?` it would reach the client as
        // an internal error, which clients retry on every other node.
        let limit = check_validation_result_with_data!(limit.map_or(
            Ok(config.default_query_limit),
            |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.default_query_limit
                {
                    Err(QueryError::Query(QuerySyntaxError::InvalidLimit(format!(
                        "limit {} out of bounds of [1, {}]",
                        limit_value, config.default_query_limit
                    ))))
                } else {
                    Ok(limit_value as u16)
                }
            }
        ));

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
    use crate::query::tests::setup_platform_with_token_state;
    use crate::query::tests::{
        assert_drive_part_of_proof_verified, assert_invalid_argument_status, setup_platform,
        setup_platform_with_default_query_limit, NoQuorumKeysContextProvider, NO_LIMIT_PROOF_CASES,
    };
    use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::get_token_pre_programmed_distributions_response_v0;
    use dapi_grpc::platform::v0::{
        get_token_pre_programmed_distributions_request,
        get_token_pre_programmed_distributions_response, GetTokenPreProgrammedDistributionsRequest,
        GetTokenPreProgrammedDistributionsResponse,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::prelude::TimestampMillis;
    use drive::drive::Drive;
    use drive_proof_verifier::types::TokenPreProgrammedDistributions;
    use drive_proof_verifier::FromProof;
    use std::collections::BTreeMap;

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
    fn should_refuse_a_zero_limit_as_invalid_argument() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        for prove in [false, true] {
            let request = GetTokenPreProgrammedDistributionsRequestV0 {
                token_id: vec![0; 32],
                start_at_info: None,
                limit: Some(0),
                prove,
            };

            assert_invalid_argument_status(
                platform.query_token_pre_programmed_distributions_v0(request, &state, version),
            );
        }
    }

    #[test]
    fn should_refuse_a_limit_over_u16_as_invalid_argument() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        for prove in [false, true] {
            let request = GetTokenPreProgrammedDistributionsRequestV0 {
                token_id: vec![0; 32],
                start_at_info: None,
                limit: Some((u16::MAX as u32) + 1),
                prove,
            };

            assert_invalid_argument_status(
                platform.query_token_pre_programmed_distributions_v0(request, &state, version),
            );
        }
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

    /// Drive answers a proved request naming no limit with its default page and proves that
    /// page, so the client verifies the proof against the page Drive proved, from a node left at
    /// the default as from one whose operator set a larger default
    #[test]
    fn should_verify_the_default_page_of_distributions_requested_with_no_limit() {
        for (default_query_limit, stored) in NO_LIMIT_PROOF_CASES {
            let (platform, state, version) =
                setup_platform_with_default_query_limit(None, default_query_limit);

            let contract = DataContract::V1(DataContractV1 {
                id: Identifier::from([1u8; 32]),
                version: 0,
                owner_id: Identifier::from([2u8; 32]),
                document_types: Default::default(),
                config: DataContractConfig::default_for_version(version)
                    .expect("expected a contract config"),
                schema_defs: None,
                created_at: None,
                updated_at: None,
                created_at_block_height: None,
                updated_at_block_height: None,
                created_at_epoch: None,
                updated_at_epoch: None,
                groups: Default::default(),
                tokens: BTreeMap::from([(
                    0,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                )]),
                keywords: Vec::new(),
                description: None,
            });
            platform
                .drive
                .insert_contract(&contract, BlockInfo::genesis(), true, None, version)
                .expect("expected to insert contract");
            let token_id = contract.token_id(0).expect("expected token at position 0");

            // Released 25 recipients at a time
            let mut distributions = BTreeMap::<TimestampMillis, BTreeMap<Identifier, u64>>::new();
            for n in 0..stored as u64 {
                let mut recipient = [0u8; 32];
                recipient[..8].copy_from_slice(&n.to_be_bytes());
                distributions
                    .entry(1_000 * (n / 25 + 1))
                    .or_default()
                    .insert(Identifier::from(recipient), 1 + n);
            }
            let distribution =
                TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 {
                    distributions,
                });
            let mut batch_operations = vec![];
            platform
                .drive
                .add_pre_programmed_distributions(
                    token_id.to_buffer(),
                    contract.owner_id().to_buffer(),
                    &distribution,
                    &BlockInfo::genesis(),
                    &mut None,
                    &mut batch_operations,
                    None,
                    version,
                )
                .expect("expected to add pre-programmed distributions");
            platform
                .drive
                .apply_batch_low_level_drive_operations(
                    None,
                    None,
                    batch_operations,
                    &mut vec![],
                    &version.drive,
                )
                .expect("expected to apply batch operations");

            let request_v0 = GetTokenPreProgrammedDistributionsRequestV0 {
                token_id: token_id.to_vec(),
                start_at_info: None,
                limit: None,
                prove: true,
            };
            let response_v0 = platform
                .query_token_pre_programmed_distributions_v0(request_v0.clone(), &state, version)
                .expect("expected query to succeed")
                .into_data()
                .expect("expected query to be valid");
            let Some(get_token_pre_programmed_distributions_response_v0::Result::Proof(proof)) =
                &response_v0.result
            else {
                panic!("expected a proof")
            };

            // Drive proved its default page
            let (_, proved): (_, BTreeMap<TimestampMillis, BTreeMap<Identifier, u64>>) =
                Drive::verify_token_pre_programmed_distributions(
                    &proof.grovedb_proof,
                    token_id.to_buffer(),
                    None,
                    Some(default_query_limit),
                    false,
                    version,
                )
                .expect("expected the proof to verify as the node's default page");
            assert_eq!(
                proved.values().map(BTreeMap::len).sum::<usize>(),
                stored.min(default_query_limit) as usize
            );

            let error = <TokenPreProgrammedDistributions as FromProof<
                GetTokenPreProgrammedDistributionsRequest,
            >>::maybe_from_proof_with_metadata(
                GetTokenPreProgrammedDistributionsRequest {
                    version: Some(get_token_pre_programmed_distributions_request::Version::V0(
                        request_v0,
                    )),
                },
                GetTokenPreProgrammedDistributionsResponse {
                    version: Some(
                        get_token_pre_programmed_distributions_response::Version::V0(response_v0),
                    ),
                },
                Network::Testnet,
                version,
                &NoQuorumKeysContextProvider,
            )
            .expect_err("expected no quorum key");
            assert_drive_part_of_proof_verified(error);
        }
    }
}
