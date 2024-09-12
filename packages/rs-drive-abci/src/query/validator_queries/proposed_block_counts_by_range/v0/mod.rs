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
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

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
            .ok_or(drive::error::Error::Query(QuerySyntaxError::InvalidLimit(
                format!(
                    "limit {} greater than max limit {} or was set as 0",
                    limit.unwrap(),
                    config.max_query_limit
                ),
            )))?;

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
                    ProposerQueryType::ByRange(Some(limit), formatted_start),
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
