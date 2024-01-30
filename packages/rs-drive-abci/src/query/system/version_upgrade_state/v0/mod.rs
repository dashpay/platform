use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{
    get_protocol_version_upgrade_state_response, GetProtocolVersionUpgradeStateResponse, Proof,
};
use dpp::check_validation_result_with_data;

use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_request::GetProtocolVersionUpgradeStateRequestV0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::{VersionEntry, Versions};
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::GetProtocolVersionUpgradeStateResponseV0;

impl<C> Platform<C> {
    pub(super) fn query_version_upgrade_state_v0(
        &self,
        state: &PlatformState,
        request: GetProtocolVersionUpgradeStateRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetProtocolVersionUpgradeStateRequestV0 { prove } = request;

        let response_data = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .fetch_proved_versions_with_counter(None, &platform_version.drive));

            GetProtocolVersionUpgradeStateResponse {
                version: Some(get_protocol_version_upgrade_state_response::Version::V0(
                    GetProtocolVersionUpgradeStateResponseV0 {
                        result: Some(
                            get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::Result::Proof(
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
            let drive_cache = self.drive.cache.read().unwrap();
            let versions = drive_cache
                .protocol_versions_counter
                .global_cache
                .iter()
                .map(|(protocol_version, count)| VersionEntry {
                    version_number: *protocol_version,
                    vote_count: *count as u32,
                })
                .collect();

            GetProtocolVersionUpgradeStateResponse {
                version: Some(get_protocol_version_upgrade_state_response::Version::V0(
                    GetProtocolVersionUpgradeStateResponseV0 {
                        result: Some(
                            get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::Result::Versions(
                                Versions { versions },
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
