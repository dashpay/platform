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
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl<C> Platform<C> {
    pub(super) fn query_proposed_block_counts_by_evonode_ids_v0(
        &self,
        GetEvonodesProposedEpochBlocksByIdsRequestV0 { epoch, ids, prove }: GetEvonodesProposedEpochBlocksByIdsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetEvonodesProposedEpochBlocksResponseV0>, Error> {
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

            GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(
                    get_evonodes_proposed_epoch_blocks_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                .map(|(pro_tx_hash, count)| EvonodeProposedBlocks { pro_tx_hash, count })
                .collect();

            let evonode_proposed_blocks = EvonodesProposedBlocks {
                evonodes_proposed_block_counts,
            };

            GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(evonode_proposed_blocks)),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
