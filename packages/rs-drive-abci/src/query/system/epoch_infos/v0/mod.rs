use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_response::get_epochs_info_response_v0::EpochInfos;
use dapi_grpc::platform::v0::get_epochs_info_response::{
    get_epochs_info_response_v0, GetEpochsInfoResponseV0,
};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::check_validation_result_with_data;

use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_epoch_infos_v0(
        &self,
        GetEpochsInfoRequestV0 {
            start_epoch,
            count,
            ascending,
            prove,
        }: GetEpochsInfoRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetEpochsInfoResponseV0>, Error> {
        let state = self.state.read().unwrap();

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

        let metadata = ResponseMetadata {
            height: state.last_committed_height(),
            core_chain_locked_height: state.last_committed_core_height(),
            epoch: state.last_committed_block_epoch().index as u32,
            time_ms: state.last_committed_block_time_ms().unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: state.current_protocol_version_in_consensus(),
        };

        let response = if prove {
            let mut proof_response = Proof {
                grovedb_proof: Vec::new(),
                quorum_hash: state.last_committed_quorum_hash().to_vec(),
                quorum_type: self.config.validator_set_quorum_type() as u32,
                block_id_hash: state.last_committed_block_id_hash().to_vec(),
                signature: state.last_committed_block_signature().to_vec(),
                round: state.last_committed_block_round(),
            };

            drop(state);

            proof_response.grovedb_proof =
                check_validation_result_with_data!(self.drive.prove_epochs_infos(
                    start_epoch as u16,
                    count as u16,
                    ascending,
                    None,
                    platform_version
                ));

            GetEpochsInfoResponseV0 {
                result: Some(get_epochs_info_response_v0::Result::Proof(proof_response)),
                metadata: Some(metadata),
            }
        } else {
            drop(state);

            let result = check_validation_result_with_data!(self.drive.get_epochs_infos(
                start_epoch as u16,
                count as u16,
                ascending,
                None,
                platform_version
            ));

            let epoch_infos = result
                .into_iter()
                .map(|epoch_info| get_epochs_info_response_v0::EpochInfo {
                    number: epoch_info.index() as u32,
                    first_block_height: epoch_info.first_block_height(),
                    first_core_block_height: epoch_info.first_core_block_height(),
                    start_time: epoch_info.first_block_time(),
                    fee_multiplier: epoch_info.fee_multiplier(),
                })
                .collect();

            GetEpochsInfoResponseV0 {
                result: Some(get_epochs_info_response_v0::Result::Epochs(EpochInfos {
                    epoch_infos,
                })),
                metadata: Some(metadata),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;

    #[test]
    fn test_query_empty_epoch_infos() {
        let (platform, version) = setup_platform();

        let request = GetEpochsInfoRequestV0 {
            start_epoch: None, // 0
            count: 5,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_epoch_infos_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetEpochsInfoResponseV0 {
                result: Some(get_epochs_info_response_v0::Result::Epochs(EpochInfos { epoch_infos })),
                metadata: Some(_),
            }) if epoch_infos.is_empty()
        ));
    }

    #[test]
    fn test_query_empty_epoch_infos_descending() {
        let (platform, version) = setup_platform();

        let request = GetEpochsInfoRequestV0 {
            start_epoch: None, // 0
            count: 5,
            ascending: false,
            prove: false,
        };

        let validation_result = platform
            .query_epoch_infos_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            validation_result.data,
            Some(GetEpochsInfoResponseV0 {
                result: Some(get_epochs_info_response_v0::Result::Epochs(EpochInfos { epoch_infos })),
                metadata: Some(_),
            }) if epoch_infos.is_empty()
        ));
    }
}
