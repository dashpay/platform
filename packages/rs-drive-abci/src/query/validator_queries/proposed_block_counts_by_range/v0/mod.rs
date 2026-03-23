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
        let limit = limit
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.max_query_limit
                {
                    None
                } else {
                    Some(limit_value as u16)
                }
            })
            .ok_or_else(|| {
                let message = if let Some(limit) = limit {
                    format!(
                        "limit {} greater than max limit {}",
                        limit, config.max_query_limit
                    )
                } else {
                    "limit must be set in proposed block count by range query".to_string()
                };
                drive::error::Error::Query(QuerySyntaxError::InvalidLimit(message))
            })?;

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
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_limit_zero_returns_error() {
        let (platform, _state, _version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(0),
            start: None,
            prove: false,
        };

        let result = platform.query_proposed_block_counts_by_range_v0(request, &_state, _version);

        assert!(result.is_err(), "limit of 0 should return an error");
    }

    #[test]
    fn test_query_limit_exceeds_max_returns_error() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let over_limit = platform.platform.config.drive.max_query_limit as u32 + 1;
        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(over_limit),
            start: None,
            prove: false,
        };

        let result = platform.query_proposed_block_counts_by_range_v0(request, &state, version);

        assert!(result.is_err(), "limit over max should return an error");
    }

    #[test]
    fn test_query_limit_exceeds_u16_max_returns_error() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByRangeRequestV0 {
            epoch: Some(0),
            limit: Some(u16::MAX as u32 + 1),
            start: None,
            prove: false,
        };

        let result = platform.query_proposed_block_counts_by_range_v0(request, &state, version);

        assert!(
            result.is_err(),
            "limit over u16::MAX should return an error"
        );
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
}
