use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_nullifiers_branch_state_request::GetNullifiersBranchStateRequestV0;
use dapi_grpc::platform::v0::get_nullifiers_branch_state_response::GetNullifiersBranchStateResponseV0;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_nullifiers_branch_state_v0(
        &self,
        GetNullifiersBranchStateRequestV0 {
            pool_type,
            pool_identifier,
            key,
            depth,
            checkpoint_height,
        }: GetNullifiersBranchStateRequestV0,
        _platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetNullifiersBranchStateResponseV0>, Error> {
        let pool_id = if pool_identifier.is_empty() {
            None
        } else {
            Some(pool_identifier)
        };

        let merk_proof = self.drive.prove_nullifiers_branch_query(
            pool_type,
            pool_id,
            key,
            depth as u8,
            checkpoint_height,
            platform_version,
        )?;

        let response = GetNullifiersBranchStateResponseV0 { merk_proof };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
