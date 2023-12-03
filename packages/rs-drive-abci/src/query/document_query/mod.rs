use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::Version;
use dapi_grpc::platform::v0::GetDocumentsRequest;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;

use dpp::version::PlatformVersion;
use prost::Message;

mod v0;

impl<C> Platform<C> {
    /// Querying of documents
    pub(in crate::query) fn query_documents(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let GetDocumentsRequest { version } =
            check_validation_result_with_data!(GetDocumentsRequest::decode(query_data).map_err(
                |e| QueryError::InvalidArgument(format!("invalid query proto message: {}", e))
            ));

        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode data contracts query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version.drive_abci.query.document_query;

        let feature_version = match &version {
            Version::V0(_) => 0,
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
            Version::V0(get_documents_request) => {
                self.query_documents_v0(state, get_documents_request, platform_version)
            }
        }
    }
}
