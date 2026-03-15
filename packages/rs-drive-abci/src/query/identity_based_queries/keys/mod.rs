use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_keys_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identity_keys_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetIdentityKeysRequest, GetIdentityKeysResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity's keys
    pub fn query_keys(
        &self,
        GetIdentityKeysRequest { version }: GetIdentityKeysRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityKeysResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity keys query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .keys;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "keys".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_keys_v0(request_v0, platform_state, platform_version)?;

                Ok(result.map(|response_v0| GetIdentityKeysResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
    use dapi_grpc::platform::v0::key_request_type::Request;
    use dapi_grpc::platform::v0::{AllKeys, KeyRequestType};
    use dpp::dashcore::Network;

    #[test]
    fn test_query_keys_with_none_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequest { version: None };

        let result = platform
            .query_keys(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("could not decode identity keys query")
        ));
    }

    #[test]
    fn test_query_keys_with_valid_v0_version() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequest {
            version: Some(RequestVersion::V0(GetIdentityKeysRequestV0 {
                identity_id: vec![0; 32],
                request_type: Some(KeyRequestType {
                    request: Some(Request::AllKeys(AllKeys {})),
                }),
                limit: None,
                offset: None,
                prove: false,
            })),
        };

        let result = platform
            .query_keys(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        let response = result.data.expect("expected response data");
        assert!(matches!(response.version, Some(ResponseVersion::V0(_))));
    }
}
