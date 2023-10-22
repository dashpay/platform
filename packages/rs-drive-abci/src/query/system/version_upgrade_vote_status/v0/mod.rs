use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_version_upgrade_vote_status_response::{
    VersionSignal, VersionSignals,
};
use dapi_grpc::platform::v0::{
    get_version_upgrade_vote_status_response, GetVersionUpgradeVoteStatusRequest,
    GetVersionUpgradeVoteStatusResponse, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_version_upgrade_vote_status_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetVersionUpgradeVoteStatusRequest {
            start_pro_tx_hash,
            count,
            prove,
        } = check_validation_result_with_data!(GetVersionUpgradeVoteStatusRequest::decode(
            query_data
        ));

        let start_pro_tx_hash: Option<Bytes32> = if start_pro_tx_hash.is_empty() {
            None
        } else {
            Some(start_pro_tx_hash.try_into()?)
        };

        let response_data = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .fetch_proved_validator_version_votes(
                    start_pro_tx_hash,
                    count,
                    None,
                    &platform_version.drive
                ));

            GetVersionUpgradeVoteStatusResponse {
                result: Some(get_version_upgrade_vote_status_response::Result::Proof(
                    Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_block_id_hash().to_vec(),
                        signature: state.last_block_signature().to_vec(),
                        round: state.last_block_round(),
                    },
                )),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        } else {
            let result =
                check_validation_result_with_data!(self.drive.fetch_validator_version_votes(
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

            GetVersionUpgradeVoteStatusResponse {
                result: Some(get_version_upgrade_vote_status_response::Result::Versions(
                    VersionSignals {
                        version_signals: versions,
                    },
                )),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
