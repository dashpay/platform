use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_vote_polls_by_end_date_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of the contested vote polls by a query targeting the end date
    /// This is for querying what votes are ending soon, but can be for other time based queries
    pub fn query_vote_polls_by_end_date_query(
        &self,
        GetVotePollsByEndDateRequest { version }: GetVotePollsByEndDateRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetVotePollsByEndDateResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode contested vote polls by end date query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .voting_based_queries
            .vote_polls_by_end_date_query;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "vote_polls_by_end_date_query".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_vote_polls_by_end_date_query_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0| GetVotePollsByEndDateResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
