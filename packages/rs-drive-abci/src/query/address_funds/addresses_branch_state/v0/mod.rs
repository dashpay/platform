use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_addresses_branch_state_request::GetAddressesBranchStateRequestV0;
use dapi_grpc::platform::v0::get_addresses_branch_state_response::GetAddressesBranchStateResponseV0;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_addresses_branch_state_v0(
        &self,
        GetAddressesBranchStateRequestV0 {
            key,
            depth,
            checkpoint_height,
        }: GetAddressesBranchStateRequestV0,
        _platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesBranchStateResponseV0>, Error> {
        // checkpoint_height is now required and must match the height from trunk response metadata
        let merk_proof = self.drive.prove_address_funds_branch_query(
            key,
            depth as u8,
            checkpoint_height,
            platform_version,
        )?;

        let response = GetAddressesBranchStateResponseV0 { merk_proof };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
