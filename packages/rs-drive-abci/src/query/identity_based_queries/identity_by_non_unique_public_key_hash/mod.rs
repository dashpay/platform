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
            RequestVersion::V1(_) => 1,
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
                let request = self.query_identity_by_non_unique_public_key_hash_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    request.map(|response_v0| GetIdentityByNonUniquePublicKeyHashResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
            RequestVersion::V1(request_v1) => {
                let request = self.query_identity_by_non_unique_public_key_hash_v1(
                    request_v1,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    request.map(|response_v1| GetIdentityByNonUniquePublicKeyHashResponse {
                        version: Some(ResponseVersion::V1(response_v1)),
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
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::{
        GetIdentityByNonUniquePublicKeyHashRequestV0, GetIdentityByNonUniquePublicKeyHashRequestV1,
    };
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::get_identity_by_non_unique_public_key_hash_response_v0::Result as V0Result;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::get_identity_by_non_unique_public_key_hash_response_v1::Result as V1Result;
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

    fn request(version: u8, prove: bool) -> GetIdentityByNonUniquePublicKeyHashRequest {
        let (public_key_hash, start_after) = (vec![0; 20], None);
        GetIdentityByNonUniquePublicKeyHashRequest {
            version: Some(match version {
                0 => RequestVersion::V0(GetIdentityByNonUniquePublicKeyHashRequestV0 {
                    public_key_hash,
                    start_after,
                    prove,
                }),
                _ => RequestVersion::V1(GetIdentityByNonUniquePublicKeyHashRequestV1 {
                    public_key_hash,
                    start_after,
                    prove,
                }),
            }),
        }
    }

    /// A V0 request is answered in the 4.1 shape at every protocol version: separate index
    /// and identity proofs in result field 2. Clients that know only V0, and SDKs that have
    /// not yet verified a protocol 14 answer, rely on that.
    #[test]
    fn should_answer_v0_with_separate_proofs_at_every_served_version() {
        for protocol in 12..=PlatformVersion::latest().protocol_version {
            let (platform, state, version) = setup_platform(None, Network::Testnet, Some(protocol));
            let response = platform
                .query_identity_by_non_unique_public_key_hash(request(0, true), &state, version)
                .expect("expected query to succeed")
                .into_data()
                .expect("expected an answer");
            let wire = GetIdentityByNonUniquePublicKeyHashResponse::decode(
                response.encode_to_vec().as_slice(),
            )
            .expect("decodes");
            let Some(ResponseVersion::V0(v0)) = wire.version else {
                panic!("PV{protocol}: a V0 request must get a V0 response")
            };
            assert_eq!(v0.metadata.expect("metadata").protocol_version, protocol);
            let Some(V0Result::Proof(proved)) = v0.result else {
                panic!("PV{protocol}: a proved V0 answer carries separate proofs")
            };
            assert!(proved.grovedb_identity_public_key_hash_proof.is_some());
            // No identity uses this key, so only the index proof is sent.
            assert_eq!(proved.identity_proof_bytes, None);
        }
    }

    #[test]
    fn should_answer_v1_with_one_proof_from_protocol_14() {
        for protocol in 14..=PlatformVersion::latest().protocol_version {
            let (platform, state, version) = setup_platform(None, Network::Testnet, Some(protocol));
            let response = platform
                .query_identity_by_non_unique_public_key_hash(request(1, true), &state, version)
                .expect("expected query to succeed")
                .into_data()
                .expect("expected an answer");
            let wire = GetIdentityByNonUniquePublicKeyHashResponse::decode(
                response.encode_to_vec().as_slice(),
            )
            .expect("decodes");
            let Some(ResponseVersion::V1(v1)) = wire.version else {
                panic!("PV{protocol}: a V1 request must get a V1 response")
            };
            assert_eq!(v1.metadata.expect("metadata").protocol_version, protocol);
            assert!(
                matches!(v1.result, Some(V1Result::Proof(_))),
                "PV{protocol}: a proved V1 answer carries one proof"
            );
        }
    }

    /// Protocol 13 nodes refuse V1, as 4.1 nodes do, so a network below protocol 14
    /// answers the same requests whichever release its nodes run.
    #[test]
    fn should_refuse_v1_below_protocol_14() {
        for protocol in [12, 13] {
            let (platform, state, version) = setup_platform(None, Network::Testnet, Some(protocol));
            for prove in [false, true] {
                let result = platform
                    .query_identity_by_non_unique_public_key_hash(
                        request(1, prove),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed");
                assert!(
                    matches!(
                        result.errors.as_slice(),
                        [QueryError::UnsupportedQueryVersion(_, 0, 0, p, 1)] if *p == protocol
                    ),
                    "PV{protocol}: {:?}",
                    result.errors
                );
            }
        }
    }

    #[test]
    fn should_answer_unproved_v0_and_v1_alike() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let answer = |version_number| {
            platform
                .query_identity_by_non_unique_public_key_hash(
                    request(version_number, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed")
                .into_data()
                .expect("expected an answer")
                .version
        };
        let (Some(ResponseVersion::V0(v0)), Some(ResponseVersion::V1(v1))) = (answer(0), answer(1))
        else {
            panic!("each request version gets its own response version")
        };
        let Some(V0Result::Identity(v0_identity)) = v0.result else {
            panic!("unproved V0 answer")
        };
        let Some(V1Result::Identity(v1_identity)) = v1.result else {
            panic!("unproved V1 answer")
        };
        assert_eq!(v0_identity.identity, v1_identity.identity);
        assert_eq!(v0.metadata, v1.metadata);
    }
}
