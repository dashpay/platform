use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;

use dapi_grpc::platform::v0::get_finalized_epoch_infos_request::GetFinalizedEpochInfosRequestV0;
use dapi_grpc::platform::v0::get_finalized_epoch_infos_response::{
    get_finalized_epoch_infos_response_v0, GetFinalizedEpochInfosResponseV0,
};
use dapi_grpc::platform::v0::get_finalized_epoch_infos_response::get_finalized_epoch_infos_response_v0::{BlockProposer, FinalizedEpochInfo, FinalizedEpochInfos};
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::check_validation_result_with_data;
use dpp::version::PlatformVersion;
use dpp::validation::ValidationResult;

impl<C> Platform<C> {
    pub(super) fn query_finalized_epoch_infos_v0(
        &self,
        GetFinalizedEpochInfosRequestV0 {
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
            prove,
        }: GetFinalizedEpochInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetFinalizedEpochInfosResponseV0>, Error> {
        // Validate that epoch indices fit in u16
        if start_epoch_index > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "start_epoch_index {} exceeds maximum value of {}",
                    start_epoch_index,
                    u16::MAX
                )),
            ));
        }

        if end_epoch_index > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "end_epoch_index {} exceeds maximum value of {}",
                    end_epoch_index,
                    u16::MAX
                )),
            ));
        }

        if end_epoch_index == start_epoch_index
            && (!end_epoch_index_included || !start_epoch_index_included)
        {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    format!("when start_epoch_index equals end_epoch_index ({}), both boundaries must be included to return a result", start_epoch_index),
                ),
            ));
        }

        if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_finalized_epoch_infos(
                    start_epoch_index as u16,
                    start_epoch_index_included,
                    end_epoch_index as u16,
                    end_epoch_index_included,
                    None,
                    platform_version,
                )
                .map_err(QueryError::Drive));

            Ok(QueryValidationResult::new_with_data(
                GetFinalizedEpochInfosResponseV0 {
                    result: Some(get_finalized_epoch_infos_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    )),
                    metadata: Some(self.response_metadata_v0(platform_state)),
                },
            ))
        } else {
            let finalized_epoch_infos: Vec<_> = check_validation_result_with_data!(self
                .drive
                .get_finalized_epoch_infos(
                    start_epoch_index as u16,
                    start_epoch_index_included,
                    end_epoch_index as u16,
                    end_epoch_index_included,
                    None,
                    platform_version,
                )
                .map_err(QueryError::Drive));

            let finalized_epoch_infos: Vec<FinalizedEpochInfo> = finalized_epoch_infos
                .into_iter()
                .map(|(epoch_index, finalized_info)| {
                    // Convert the BTreeMap to the proto repeated field format
                    let block_proposers = finalized_info
                        .block_proposers()
                        .iter()
                        .map(|(id, count)| BlockProposer {
                            proposer_id: id.to_vec(),
                            block_count: *count as u32,
                        })
                        .collect();

                    FinalizedEpochInfo {
                        number: epoch_index as u32,
                        first_block_height: finalized_info.first_block_height(),
                        first_core_block_height: finalized_info.first_core_block_height(),
                        first_block_time: finalized_info.first_block_time(),
                        fee_multiplier: finalized_info.fee_multiplier_permille() as f64 / 1000.0,
                        protocol_version: finalized_info.protocol_version(),
                        total_blocks_in_epoch: finalized_info.total_blocks_in_epoch(),
                        next_epoch_start_core_block_height: finalized_info
                            .next_epoch_start_core_block_height(),
                        total_processing_fees: finalized_info.total_processing_fees(),
                        total_distributed_storage_fees: finalized_info
                            .total_distributed_storage_fees(),
                        total_created_storage_fees: finalized_info.total_created_storage_fees(),
                        core_block_rewards: finalized_info.core_block_rewards(),
                        block_proposers,
                    }
                })
                .collect();

            Ok(QueryValidationResult::new_with_data(
                GetFinalizedEpochInfosResponseV0 {
                    result: Some(get_finalized_epoch_infos_response_v0::Result::Epochs(
                        FinalizedEpochInfos {
                            finalized_epoch_infos,
                        },
                    )),
                    metadata: Some(self.response_metadata_v0(platform_state)),
                },
            ))
        }
    }
}
