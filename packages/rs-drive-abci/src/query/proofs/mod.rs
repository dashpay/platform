use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_proofs_request::Version;
use dapi_grpc::platform::v0::GetProofsRequest;
use dapi_grpc::Message;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of platform proofs
    pub(in crate::query) fn query_proofs(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let GetProofsRequest { version } =
            check_validation_result_with_data!(GetProofsRequest::decode(query_data).map_err(|e| {
                QueryError::InvalidArgument(format!("invalid query proto message: {}", e))
            }));

        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity keys query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version.drive_abci.query.proofs_query;

        let feature_version = match &version {
            Version::V0(_) => 0,
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
            Version::V0(get_identity_request) => {
                self.query_proofs_v0(state, get_identity_request, platform_version)
            }
        }
    }
}
