use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
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
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;
use crate::platform_types::platform_state::PlatformStateV0Methods;

impl<C> Platform<C> {
    pub(super) fn query_proposed_block_counts_by_evonode_ids_v0(
        &self,
        GetEvonodesProposedEpochBlocksByIdsRequestV0 { epoch, ids, prove }: GetEvonodesProposedEpochBlocksByIdsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetEvonodesProposedEpochBlocksResponseV0>, Error> {
        if ids.len() > self.config.drive.max_query_limit as usize {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::TooManyElements(format!(
                    "this query only supports up to {} ids at a time",
                    ids.len()
                )),
            ));
        }
        let evonode_ids = check_validation_result_with_data!(ids
            .into_iter()
            .map(|evonode_id_vec| {
                if evonode_id_vec.len() != 32 {
                    Err(QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string(),
                    ))
                } else {
                    Ok(evonode_id_vec)
                }
            })
            .collect::<Result<Vec<Vec<u8>>, QueryError>>());

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
                ProposerQueryType::ByIds(evonode_ids),
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
                    ProposerQueryType::ByIds(evonode_ids),
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
    fn test_query_too_many_ids() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let ids: Vec<Vec<u8>> = (0..=platform.platform.config.drive.max_query_limit)
            .map(|i| {
                let mut id = [0u8; 32];
                id[0] = (i & 0xFF) as u8;
                id[1] = ((i >> 8) & 0xFF) as u8;
                id.to_vec()
            })
            .collect();

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(0),
            ids,
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::TooManyElements(msg)] if msg.contains("ids at a time")
        ));
    }

    #[test]
    fn test_query_invalid_id_length() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(0),
            ids: vec![vec![1, 2, 3]], // too short
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("32 bytes")
        ));
    }

    #[test]
    fn test_query_epoch_too_high() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(u16::MAX as u32),
            ids: vec![vec![0u8; 32]],
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("epoch must be within a normal range")
        ));
    }

    #[test]
    fn test_query_no_epoch_uses_current_prove() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: None,
            ids: vec![vec![0u8; 32]],
            prove: true,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
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
    fn test_query_empty_ids_non_prove() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(0),
            ids: vec![],
            prove: false,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
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
    fn test_query_multiple_ids_prove() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(0),
            ids: vec![vec![1u8; 32], vec![2u8; 32]],
            prove: true,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
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
    fn test_query_valid_ids_prove() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(0),
            ids: vec![vec![1u8; 32]],
            prove: true,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
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
    fn test_query_with_explicit_epoch_prove() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetEvonodesProposedEpochBlocksByIdsRequestV0 {
            epoch: Some(5),
            ids: vec![vec![0u8; 32]],
            prove: true,
        };

        let result = platform
            .query_proposed_block_counts_by_evonode_ids_v0(request, &state, version)
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
