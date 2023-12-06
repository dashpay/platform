use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{
    get_protocol_version_upgrade_vote_status_response, GetProtocolVersionUpgradeVoteStatusResponse,
    Proof,
};
use dpp::check_validation_result_with_data;

use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::{VersionSignal, VersionSignals};
use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_response::GetProtocolVersionUpgradeVoteStatusResponseV0;
use crate::error::query::QueryError;

impl<C> Platform<C> {
    pub(super) fn query_version_upgrade_vote_status_v0(
        &self,
        state: &PlatformState,
        request: GetProtocolVersionUpgradeVoteStatusRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetProtocolVersionUpgradeVoteStatusRequestV0 {
            start_pro_tx_hash,
            count,
            prove,
        } = request;

        let start_pro_tx_hash: Option<[u8; 32]> = if start_pro_tx_hash.is_empty() {
            None
        } else {
            match start_pro_tx_hash.try_into() {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(format!(
                            "start_pro_tx_hash not 32 bytes long, received {}",
                            hex::encode(e)
                        )),
                    ))
                }
            }
        };

        if count >= u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!("count too high, received {}", count)),
            ));
        }

        let response_data = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .fetch_proved_validator_version_votes(
                    start_pro_tx_hash,
                    count as u16,
                    None,
                    &platform_version.drive
                ));

            GetProtocolVersionUpgradeVoteStatusResponse {
                version: Some(get_protocol_version_upgrade_vote_status_response::Version::V0(
                    GetProtocolVersionUpgradeVoteStatusResponseV0 {
                        result: Some(
                            get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::Result::Proof(
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
            let result =
                check_validation_result_with_data!(self.drive.fetch_validator_version_votes(
                    start_pro_tx_hash,
                    count as u16,
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

            GetProtocolVersionUpgradeVoteStatusResponse {
                version: Some(get_protocol_version_upgrade_vote_status_response::Version::V0(
                    GetProtocolVersionUpgradeVoteStatusResponseV0 {
                        result: Some(
                            get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::Result::Versions(
                                VersionSignals {
                                    version_signals: versions,
                                },
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
