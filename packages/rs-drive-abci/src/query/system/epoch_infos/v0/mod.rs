use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_response::get_epochs_info_response_v0::EpochInfos;
use dapi_grpc::platform::v0::get_epochs_info_response::GetEpochsInfoResponseV0;
use dapi_grpc::platform::v0::{get_epochs_info_response, GetEpochsInfoResponse, Proof};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::check_validation_result_with_data;

use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_epoch_infos_v0(
        &self,
        state: &PlatformState,
        request: GetEpochsInfoRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetEpochsInfoRequestV0 {
            start_epoch,
            count,
            ascending,
            prove,
        } = request;

        let start_epoch = start_epoch.unwrap_or_else(|| {
            if ascending {
                0
            } else {
                state.last_committed_block_epoch_ref().index as u32
            }
        });

        if start_epoch >= u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "start epoch too high, received {}",
                    start_epoch
                )),
            ));
        }

        if start_epoch + count >= u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!("count too high, received {}", count)),
            ));
        }

        let response_data = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_epochs_infos(
                start_epoch as u16,
                count as u16,
                ascending,
                None,
                platform_version
            ));

            GetEpochsInfoResponse {
                version: Some(get_epochs_info_response::Version::V0(
                    GetEpochsInfoResponseV0 {
                        result: Some(
                            get_epochs_info_response::get_epochs_info_response_v0::Result::Proof(
                                Proof {
                                    grovedb_proof: proof,
                                    quorum_hash: state.last_committed_quorum_hash().to_vec(),
                                    quorum_type,
                                    block_id_hash: state.last_committed_block_id_hash().to_vec(),
                                    signature: state.last_committed_block_signature().to_vec(),
                                    round: state.last_committed_block_round(),
                                },
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
        } else {
            let result = check_validation_result_with_data!(self.drive.get_epochs_infos(
                start_epoch as u16,
                count as u16,
                ascending,
                None,
                platform_version
            ));
            let epoch_infos = result
                .into_iter()
                .map(|epoch_info| {
                    get_epochs_info_response::get_epochs_info_response_v0::EpochInfo {
                        number: epoch_info.index() as u32,
                        first_block_height: epoch_info.first_block_height(),
                        first_core_block_height: epoch_info.first_core_block_height(),
                        start_time: epoch_info.first_block_time(),
                        fee_multiplier: epoch_info.fee_multiplier(),
                    }
                })
                .collect();

            GetEpochsInfoResponse {
                version: Some(get_epochs_info_response::Version::V0(
                    GetEpochsInfoResponseV0 {
                        result: Some(
                            get_epochs_info_response::get_epochs_info_response_v0::Result::Epochs(
                                EpochInfos { epoch_infos },
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
