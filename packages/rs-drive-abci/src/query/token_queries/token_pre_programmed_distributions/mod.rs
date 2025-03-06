use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetTokenPreProgrammedDistributionsRequest, GetTokenPreProgrammedDistributionsResponse,
};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Querying of an identity's token infos by a public key hash
    pub fn query_token_pre_programmed_distributions(
        &self,
        GetTokenPreProgrammedDistributionsRequest { version }: GetTokenPreProgrammedDistributionsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenPreProgrammedDistributionsResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identity token infos query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .token_queries
            .token_pre_programmed_distributions;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "token_pre_programmed_distributions".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_token_pre_programmed_distributions_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(
                    result.map(|response_v0| GetTokenPreProgrammedDistributionsResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
