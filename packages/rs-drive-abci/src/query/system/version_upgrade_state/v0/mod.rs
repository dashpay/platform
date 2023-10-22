use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_version_upgrade_state_response::{VersionEntry, Versions};
use dapi_grpc::platform::v0::{
    get_data_contract_response, get_version_upgrade_state_response, GetDataContractRequest,
    GetDataContractResponse, GetIdentityBalanceResponse, GetIdentityRequest,
    GetVersionUpgradeStateRequest, GetVersionUpgradeStateResponse, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_version_upgrade_state_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetVersionUpgradeStateRequest { prove } =
            check_validation_result_with_data!(GetVersionUpgradeStateRequest::decode(query_data));

        let response_data = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .fetch_proved_versions_with_counter(None, &platform_version.drive));

            GetVersionUpgradeStateResponse {
                result: Some(get_version_upgrade_state_response::Result::Proof(Proof {
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

            GetVersionUpgradeStateResponse {
                result: Some(get_version_upgrade_state_response::Result::Versions(
                    Versions { versions },
                )),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
