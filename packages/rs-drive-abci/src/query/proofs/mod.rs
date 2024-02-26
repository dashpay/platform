use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_proofs_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_proofs_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetProofsRequest, GetProofsResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of platform proofs
    pub fn query_proofs(
        &self,
        GetProofsRequest { version }: GetProofsRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity keys query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version.drive_abci.query.proofs_query;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "proofs".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_proofs_v0(request_v0, platform_version)?;

                Ok(result.map(|response_v0| GetProofsResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
