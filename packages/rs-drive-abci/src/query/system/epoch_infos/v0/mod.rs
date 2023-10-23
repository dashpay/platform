use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_version_upgrade_state_response::{VersionEntry, Versions};
use dapi_grpc::platform::v0::{
    get_data_contract_response, get_epochs_info_response, get_version_upgrade_state_response,
    GetDataContractRequest, GetDataContractResponse, GetEpochsInfoRequest, GetEpochsInfoResponse,
    GetIdentityBalanceResponse, GetIdentityRequest, GetVersionUpgradeStateRequest,
    GetVersionUpgradeStateResponse, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_epoch_infos_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetEpochsInfoRequest {
            start_epoch,
            count,
            prove,
        } = check_validation_result_with_data!(GetEpochsInfoRequest::decode(query_data));

        let response_data = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_epochs_infos(
                start_epoch,
                count,
                None,
                &platform_version.drive
            ));

            GetEpochsInfoResponse {
                result: Some(get_epochs_info_response::Result::Proof(Proof {
                    grovedb_proof: proof,
                    quorum_hash: state.last_quorum_hash().to_vec(),
                    quorum_type,
                    block_id_hash: state.last_block_id_hash().to_vec(),
                    signature: state.last_block_signature().to_vec(),
                    round: state.last_block_round(),
                })),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        } else {
            let result = check_validation_result_with_data!(self.drive.get_epochs_infos(
                start_pro_tx_hash,
                count,
                None,
                &platform_version.drive
            ));
            let versions = result
                .into_iter()
                .map(|(pro_tx_hash, version)| VersionSignal {
                    pro_tx_hash: pro_tx_hash.to_vec(),
                    version,
                })
                .collect();

            GetEpochsInfoResponse {
                result: Some(get_epochs_info_response::Result::Epochs(VersionSignals {
                    version_signals: versions,
                })),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
