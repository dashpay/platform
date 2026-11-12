use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_response::GetEvonodesProposedEpochBlocksResponseV0;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_response::get_evonodes_proposed_epoch_blocks_response_v0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_response::get_evonodes_proposed_epoch_blocks_response_v0::{EvonodeProposedBlocks, EvonodesProposedBlocks};
use dpp::block::epoch::Epoch;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::query::proposer_block_count_query::ProposerQueryType;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;
use crate::platform_types::platform_state::PlatformStateV0Methods;

impl<C> Platform<C> {
    pub(super) fn query_proposed_block_counts_by_range_v0(
        &self,
        GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch,
            limit,
            prove,
            start,
        }: GetEvonodesProposedEpochBlocksByRangeRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetEvonodesProposedEpochBlocksResponseV0>, Error> {
        let config = &self.config.drive;
        // A refused limit is a request error: returned through `?` it would reach the client as
        // an internal error, which clients retry on every other node.
        let limit = check_validation_result_with_data!(limit.map_or(
            Ok(config.default_query_limit),
            |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.max_query_limit
                {
                    Err(QueryError::Query(QuerySyntaxError::InvalidLimit(format!(
                        "limit {} out of bounds of [1, {}]",
                        limit_value, config.max_query_limit
                    ))))
                } else {
                    Ok(limit_value as u16)
                }
            }
        ));

        let formatted_start = match start {
            None => None,
            Some(Start::StartAfter(after)) => {
                let id: [u8; 32] =
                    check_validation_result_with_data!(after.try_into().map_err(|_| {
                        QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "start after should be a 32 byte identifier",
                        ))
                    }));
                Some((id, false))
            }
            Some(Start::StartAt(after)) => {
                let id: [u8; 32] =
                    check_validation_result_with_data!(after.try_into().map_err(|_| {
                        QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "start after should be a 32 byte identifier",
                        ))
                    }));
                Some((id, true))
            }
        };

        let epoch = if let Some(epoch) = epoch {
            if epoch > (u16::MAX - 1) as u32 {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "epoch must be within a normal range (less than u16::Max - 1)".to_string(),
                    ),
                ));
            }

            let epoch =
                check_validation_result_with_data!(Epoch::new(epoch as u16).map_err(|_| {
                    QueryError::InvalidArgument(
                        "epoch must be within a normal range (less than u16::Max - 1)".to_string(),
                    )
                }));
            epoch
        } else {
            // Get current epoch instead
            platform_state.last_committed_block_epoch()
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_epoch_proposers(
                &epoch,
                ProposerQueryType::ByRange(Some(limit), formatted_start),
                None,
                platform_version
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(get_evonodes_proposed_epoch_blocks_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let evonodes_proposed_block_counts = self
                .drive
                .fetch_epoch_proposers(
                    &epoch,
                    ProposerQueryType::ByRange(Some(limit), formatted_start),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(pro_tx_hash, count)| EvonodeProposedBlocks {
                    pro_tx_hash: pro_tx_hash.to_vec(),
                    count,
                })
                .collect();

            let evonode_proposed_blocks = EvonodesProposedBlocks {
                evonodes_proposed_block_counts,
            };

            GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(evonode_proposed_blocks)),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{
        assert_drive_part_of_proof_verified, assert_invalid_argument_status, setup_platform,
        setup_platform_with_default_query_limit, NoQuorumKeysContextProvider, NO_LIMIT_PROOF_CASES,
    };
    use dapi_grpc::platform::v0::{
        get_evonodes_proposed_epoch_blocks_by_range_request,
        get_evonodes_proposed_epoch_blocks_response, GetEvonodesProposedEpochBlocksByRangeRequest,
        GetEvonodesProposedEpochBlocksResponse,
    };
    use dpp::dashcore::Network;
    use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use drive::drive::Drive;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use drive::util::batch::GroveDbOpBatch;
    use drive_proof_verifier::types::ProposerBlockCounts;
    use drive_proof_verifier::FromProof;

    #[test]
    fn should_refuse_a_zero_limit_as_invalid_argument() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        for prove in [false, true] {
            let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(0),
                limit: Some(0),
                start: None,
                prove,
            };

            assert_invalid_argument_status(
                platform.query_proposed_block_counts_by_range_v0(request, &state, version),
            );
        }
    }

    #[test]
    fn should_refuse_a_limit_over_the_max_as_invalid_argument() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);
        let over_limit = platform.platform.config.drive.max_query_limit as u32 + 1;

        for prove in [false, true] {
            let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(0),
                limit: Some(over_limit),
                start: None,
                prove,
            };

            assert_invalid_argument_status(
                platform.query_proposed_block_counts_by_range_v0(request, &state, version),
            );
        }
    }

    #[test]
    fn should_refuse_a_limit_over_u16_as_invalid_argument() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        for prove in [false, true] {
            let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(0),
                limit: Some(u16::MAX as u32 + 1),
                start: None,
                prove,
            };

            assert_invalid_argument_status(
                platform.query_proposed_block_counts_by_range_v0(request, &state, version),
            );
        }
    }

    #[test]
    fn test_query_no_limit_uses_default() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: None,
            start: None,
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(
                    get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(_)
                ),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_invalid_start_after_length() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: Some(Start::StartAfter(vec![1, 2, 3])), // not 32 bytes
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                QuerySyntaxError::InvalidStartsWithClause(_)
            )]
        ));
    }

    #[test]
    fn test_query_invalid_start_at_length() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: Some(Start::StartAt(vec![1, 2, 3, 4])), // not 32 bytes
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                QuerySyntaxError::InvalidStartsWithClause(_)
            )]
        ));
    }

    #[test]
    fn test_query_epoch_too_high() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(u16::MAX as u32),
            limit: Some(10),
            start: None,
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("epoch must be within a normal range")
        ));
    }

    #[test]
    fn test_query_no_epoch_uses_current() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: None,
            limit: Some(10),
            start: None,
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(
                    get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(_)
                ),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_non_prove_empty_results() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: None,
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        if let Some(GetEvonodesProposedEpochBlocksResponseV0 {
            result:
                Some(
                    get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(
                        info,
                    ),
                ),
            metadata: Some(_),
        }) = result.data
        {
            assert!(info.evonodes_proposed_block_counts.is_empty());
        } else {
            panic!("expected EvonodesProposedBlockCountsInfo result");
        }
    }

    #[test]
    fn test_query_prove_path() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: None,
            prove: true,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(get_evonodes_proposed_epoch_blocks_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_valid_start_after() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: Some(Start::StartAfter(vec![0u8; 32])),
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(
                    get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(_)
                ),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_valid_start_at() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: Some(Start::StartAt(vec![0u8; 32])),
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(
                    get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(_)
                ),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_valid_start_at_prove() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(10),
            start: Some(Start::StartAt(vec![0u8; 32])),
            prove: true,
        };

        let result = platform
            .query_proposed_block_counts_by_range_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(get_evonodes_proposed_epoch_blocks_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    /// Drive answers a proved request naming no limit with its default page and proves that
    /// page, so the client verifies the proof against the page Drive proved, from a node left at
    /// the default as from one whose operator set a larger default
    #[test]
    fn should_verify_the_default_page_of_proposers_requested_with_no_limit() {
        for (default_query_limit, stored) in NO_LIMIT_PROOF_CASES {
            let (platform, state, version) =
                setup_platform_with_default_query_limit(Some((1, 1)), default_query_limit);

            let epoch = Epoch::new(0).expect("expected epoch 0");
            let mut batch = GroveDbOpBatch::new();
            batch.push(epoch.init_proposers_tree_operation());
            for n in 0..stored as u64 {
                let mut pro_tx_hash = [0u8; 32];
                pro_tx_hash[..8].copy_from_slice(&n.to_be_bytes());
                batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, n + 1));
            }
            platform
                .drive
                .grove_apply_batch(batch, false, None, &version.drive)
                .expect("expected to add the proposers");

            let request_v0 = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(0),
                limit: None,
                start: None,
                prove: true,
            };
            let response_v0 = platform
                .query_proposed_block_counts_by_range_v0(request_v0.clone(), &state, version)
                .expect("expected query to succeed")
                .into_data()
                .expect("expected query to be valid");
            let Some(get_evonodes_proposed_epoch_blocks_response_v0::Result::Proof(proof)) =
                &response_v0.result
            else {
                panic!("expected a proof")
            };

            // Drive proved its default page
            let (_, proposers): (_, Vec<([u8; 32], u64)>) = Drive::verify_epoch_proposers(
                &proof.grovedb_proof,
                0,
                ProposerQueryType::ByRange(Some(default_query_limit), None),
                version,
            )
            .expect("expected the proof to verify as the node's default page");
            assert_eq!(proposers.len(), stored.min(default_query_limit) as usize);

            let error = <ProposerBlockCounts as FromProof<
                GetEvonodesProposedEpochBlocksByRangeRequest,
            >>::maybe_from_proof_with_metadata(
                GetEvonodesProposedEpochBlocksByRangeRequest {
                    version: Some(
                        get_evonodes_proposed_epoch_blocks_by_range_request::Version::V0(
                            request_v0,
                        ),
                    ),
                },
                GetEvonodesProposedEpochBlocksResponse {
                    version: Some(get_evonodes_proposed_epoch_blocks_response::Version::V0(
                        response_v0,
                    )),
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
