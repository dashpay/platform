use crate::error::execution::ExecutionError;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetIdentityByNonUniquePublicKeyHashRequest, GetIdentityByNonUniquePublicKeyHashResponse,
};
use dpp::version::PlatformVersion;

mod v0;
mod v1;

impl<C> Platform<C> {
    /// Querying of an identity by a public key hash
    pub fn query_identity_by_non_unique_public_key_hash(
        &self,
        GetIdentityByNonUniquePublicKeyHashRequest { version }: GetIdentityByNonUniquePublicKeyHashRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityByNonUniquePublicKeyHashResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identity by public key non unique hash query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .identity_by_non_unique_public_key_hash;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };

        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identity_by_non_unique_public_key_hash".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let request = match platform_version
                    .drive_abci
                    .query
                    .identity_by_non_unique_public_key_hash_handler
                {
                    0 => self.query_identity_by_non_unique_public_key_hash_v0(
                        request_v0,
                        platform_state,
                        platform_version,
                    )?,
                    1 => self.query_identity_by_non_unique_public_key_hash_v1(
                        request_v0,
                        platform_state,
                        platform_version,
                    )?,
                    version => {
                        return Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "query_identity_by_non_unique_public_key_hash".into(),
                            known_versions: vec![0, 1],
                            received: version,
                        }))
                    }
                };

                Ok(
                    request.map(|response_v0| GetIdentityByNonUniquePublicKeyHashResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::get_identity_by_non_unique_public_key_hash_response_v0::Result as LookupResult;
    use dapi_grpc::Message;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_identity_by_non_unique_public_key_hash_with_none_version() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityByNonUniquePublicKeyHashRequest { version: None };

        let result = platform
            .query_identity_by_non_unique_public_key_hash(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("could not decode identity by public key non unique hash query")
        ));
    }

    #[test]
    fn test_query_identity_by_non_unique_public_key_hash_with_valid_v0_version() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityByNonUniquePublicKeyHashRequest {
            version: Some(RequestVersion::V0(
                GetIdentityByNonUniquePublicKeyHashRequestV0 {
                    public_key_hash: vec![0; 20],
                    start_after: None,
                    prove: false,
                },
            )),
        };

        let result = platform
            .query_identity_by_non_unique_public_key_hash(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        let response = result.data.expect("expected response data");
        assert!(matches!(response.version, Some(ResponseVersion::V0(_))));
    }
    #[test]
    fn should_route_nonunique_response_generations_without_changing_request_version() {
        for protocol in [13, PlatformVersion::latest().protocol_version] {
            let (platform, state, version) = setup_platform(None, Network::Testnet, Some(protocol));
            for prove in [false, true] {
                let request = GetIdentityByNonUniquePublicKeyHashRequest {
                    version: Some(RequestVersion::V0(
                        GetIdentityByNonUniquePublicKeyHashRequestV0 {
                            public_key_hash: vec![0; 20],
                            start_after: None,
                            prove,
                        },
                    )),
                };
                let result = platform
                    .query_identity_by_non_unique_public_key_hash(request, &state, version)
                    .unwrap();
                assert!(result.is_valid());
                let response = result.data.unwrap();
                let encoded = response.encode_to_vec();
                let decoded =
                    GetIdentityByNonUniquePublicKeyHashResponse::decode(encoded.as_slice())
                        .unwrap();
                let Some(ResponseVersion::V0(v0)) = decoded.version else {
                    panic!("the server must preserve response version zero")
                };
                assert_eq!(v0.metadata.unwrap().protocol_version, protocol);
                match (prove, v0.result) {
                    (true, Some(LookupResult::LegacyProof(_))) if protocol == 13 => {}
                    (true, Some(LookupResult::Proof(_))) if protocol >= 14 => {}
                    (false, Some(LookupResult::Identity(identity))) => {
                        assert!(identity.identity.is_none())
                    }
                    _ => panic!("the server must preserve its current proof and identity variants"),
                }
            }
        }
    }
    #[test]
    fn should_reject_unknown_nonunique_handler_generation() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let mut unknown = version.clone();
        unknown
            .drive_abci
            .query
            .identity_by_non_unique_public_key_hash_handler = 255;
        let response = platform.query_identity_by_non_unique_public_key_hash(
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: vec![0; 20],
                start_after: None,
                prove: true,
            }
            .into(),
            &state,
            &unknown,
        );
        assert!(matches!(
            response,
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                received: 255,
                ..
            }))
        ));
    }
}
