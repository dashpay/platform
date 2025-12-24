use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_addresses_trunk_state_request::GetAddressesTrunkStateRequestV0;
use dapi_grpc::platform::v0::get_addresses_trunk_state_response::GetAddressesTrunkStateResponseV0;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_addresses_trunk_state_v0(
        &self,
        GetAddressesTrunkStateRequestV0 {}: GetAddressesTrunkStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesTrunkStateResponseV0>, Error> {
        let proof = check_validation_result_with_data!(self
            .drive
            .prove_address_funds_trunk_query(platform_version));

        let response = GetAddressesTrunkStateResponseV0 {
            proof: Some(self.response_proof_v0(platform_state, proof)),
            metadata: Some(self.response_metadata_v0(platform_state)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
