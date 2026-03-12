use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_nullifiers_trunk_state_request::GetNullifiersTrunkStateRequestV0;
use dapi_grpc::platform::v0::get_nullifiers_trunk_state_response::GetNullifiersTrunkStateResponseV0;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_nullifiers_trunk_state_v0(
        &self,
        GetNullifiersTrunkStateRequestV0 {
            pool_type,
            pool_identifier,
        }: GetNullifiersTrunkStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetNullifiersTrunkStateResponseV0>, Error> {
        let pool_id = if pool_identifier.is_empty() {
            None
        } else {
            Some(pool_identifier)
        };

        let proof = check_validation_result_with_data!(self.drive.prove_nullifiers_trunk_query(
            pool_type,
            pool_id,
            platform_version
        ));

        let (grovedb_used, proof) =
            self.response_proof_v0(platform_state, proof, GroveDBToUse::LatestCheckpoint)?;

        let response = GetNullifiersTrunkStateResponseV0 {
            proof: Some(proof),
            metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
