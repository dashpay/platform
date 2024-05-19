use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resources_request::GetContestedResourcesRequestV0;
use dapi_grpc::platform::v0::get_contested_resources_response::GetContestedResourcesResponseV0;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_contested_resources_v0(
        &self,
        GetContestedResourcesRequestV0 {
            contract_id,
            document_type_name,
            index_name,
            count,
            ascending,
            prove,
        }: GetContestedResourcesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourcesResponseV0>, Error> {
        todo!()
    }
}
