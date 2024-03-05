use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_data_contracts_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetDataContractsRequest, GetDataContractsResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of data contracts
    pub fn query_data_contracts(
        &self,
        GetDataContractsRequest { version }: GetDataContractsRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractsResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode data contracts query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .data_contract_based_queries
            .data_contracts;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };

        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "data_contracts".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let request = self.query_data_contracts_v0(request_v0, platform_version)?;

                Ok(request.map(|response_v0| GetDataContractsResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
