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
use dapi_grpc::Message;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_request::GetProtocolVersionUpgradeStateRequestV0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::{VersionEntry, Versions};
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::GetProtocolVersionUpgradeStateResponseV0;

impl<C> Platform<C> {
    pub(super) fn query_version_upgrade_state_v0(
        &self,
        GetProtocolVersionUpgradeStateRequestV0 { prove }: GetProtocolVersionUpgradeStateRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProtocolVersionUpgradeStateResponse>, Error> {
        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .fetch_proved_versions_with_counter(None, &platform_version.drive));

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetProtocolVersionUpgradeStateResponse {
                version: Some(get_protocol_version_upgrade_state_response::Version::V0(
                    GetProtocolVersionUpgradeStateResponseV0 {
                        result: Some(
                            get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::Result::Proof(
                                proof,
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
        } else {
            let protocol_versions_counter =
                self.drive.cache.protocol_versions_counter.read().unwrap();
            let versions = protocol_versions_counter
                .global_cache
                .iter()
                .map(|(protocol_version, count)| VersionEntry {
                    version_number: *protocol_version,
                    vote_count: *count as u32,
                })
                .collect();
            drop(protocol_versions_counter);

            GetProtocolVersionUpgradeStateResponse {
                version: Some(get_protocol_version_upgrade_state_response::Version::V0(
                    GetProtocolVersionUpgradeStateResponseV0 {
                        result: Some(
                            get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0::Result::Versions(
                                Versions { versions },
                            ),
                        ),
                        metadata: Some(self.response_metadata_v0()),
                    },
                )),
            }
        };
        Ok(QueryValidationResult::new_with_data(response))
    }
}
