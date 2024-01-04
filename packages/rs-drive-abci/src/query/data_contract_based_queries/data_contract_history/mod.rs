use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contract_history_request::Version;
use dapi_grpc::platform::v0::GetDataContractHistoryRequest;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;

use dpp::version::PlatformVersion;
use prost::Message;

mod v0;

impl<C> Platform<C> {
    /// Querying of a data contract history
    pub(in crate::query) fn query_data_contract_history(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let GetDataContractHistoryRequest { version } =
            check_validation_result_with_data!(GetDataContractHistoryRequest::decode(query_data)
                .map_err(|e| QueryError::InvalidArgument(format!(
                    "invalid query proto message: {}",
                    e
                ))));

        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode data contract query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .data_contract_based_queries
            .data_contract_history;

        let feature_version = match &version {
            Version::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "data_contract_history".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            Version::V0(get_data_contract_history_request) => self.query_data_contract_history_v0(
                state,
                get_data_contract_history_request,
                platform_version,
            ),
        }
    }
}
