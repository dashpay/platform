use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0, GetIdentityByNonUniquePublicKeyHashResponseV0,
};
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::get_identity_by_non_unique_public_key_hash_response_v0::{IdentityProvedResponse, IdentityResponse};
use dpp::check_validation_result_with_data;
use dpp::platform_value::{Bytes20, Bytes32};
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_by_non_unique_public_key_hash_v0(
        &self,
        GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash,
            start_after,
            prove,
        }: GetIdentityByNonUniquePublicKeyHashRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityByNonUniquePublicKeyHashResponseV0>, Error> {
        let public_key_hash =
            check_validation_result_with_data!(Bytes20::from_vec(public_key_hash)
                .map(|bytes| bytes.0)
                .map_err(|_| QueryError::InvalidArgument(
                    "public key hash must be 20 bytes long".to_string()
                )));

        let start_after = if let Some(start_after) = start_after {
            Some(check_validation_result_with_data!(Bytes32::from_vec(
                start_after
            )
            .map(|bytes| bytes.0)
            .map_err(|_| QueryError::InvalidArgument(
                "start_after must be 32 bytes long identity ID".to_string()
            ))))
        } else {
            None
        };

        let response = if prove {
            let proof = self
                .drive
                .prove_full_identity_by_non_unique_public_key_hash(
                    public_key_hash,
                    start_after,
                    None,
                    platform_version,
                )?;

            GetIdentityByNonUniquePublicKeyHashResponseV0 {
                result: Some(
                    get_identity_by_non_unique_public_key_hash_response_v0::Result::Proof(
                        IdentityProvedResponse {
                            grovedb_identity_public_key_hash_proof: Some(self.response_proof_v0(
                                platform_state,
                                proof.identity_id_public_key_hash_proof,
                            )),
                            identity_proof_bytes: proof.identity_proof,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let maybe_identity = self
                .drive
                .fetch_full_identity_by_non_unique_public_key_hash(
                    public_key_hash,
                    start_after,
                    None,
                    platform_version,
                )?;

            let serialized_identity = maybe_identity
                .map(|identity| {
                    identity
                        .serialize_consume_to_bytes()
                        .map_err(Error::Protocol)
                })
                .transpose()?;

            GetIdentityByNonUniquePublicKeyHashResponseV0 {
                metadata: Some(self.response_metadata_v0(platform_state)),
                result: Some(
                    get_identity_by_non_unique_public_key_hash_response_v0::Result::Identity(
                        IdentityResponse {
                            identity: serialized_identity,
                        },
                    ),
                ),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::ResponseMetadata;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_public_key_hash() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash: vec![0; 8],
            start_after: None,
            prove: false,
        };

        let result = platform
            .query_identity_by_non_unique_public_key_hash_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == &"public key hash must be 20 bytes long".to_string()
        ));
    }

    #[test]
    fn test_invalid_start_after() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let negative_tests: Vec<&[u8]> = vec![&[0u8; 4], &[0u8; 20], &[0u8; 64]];

        for test in negative_tests {
            let request = GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: vec![0; 20],
                start_after: Some(test.to_vec()),
                prove: false,
            };

            let result = platform
                .query_identity_by_non_unique_public_key_hash_v0(request, &state, version)
                .expect("expected query to succeed");

            assert!(
                matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg == &"start_after must be 32 bytes long identity ID".to_string()),
                "errors: {:?}",
                result.errors,
            );
        }
    }

    #[test]
    fn test_identity_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let public_key_hash = vec![0; 20];
        let request = GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash: public_key_hash.clone(),
            start_after: None,
            prove: false,
        };

        let result = platform
            .query_identity_by_non_unique_public_key_hash_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_eq!(
            result.data,
            Some(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                metadata: Some(ResponseMetadata {
                    height: 0,
                    core_chain_locked_height: 0,
                    epoch: 0,
                    time_ms: 0,
                    protocol_version: 9,
                    chain_id: "chain_id".to_string()
                }),
                result: Some(
                    get_identity_by_non_unique_public_key_hash_response_v0::Result::Identity(
                        IdentityResponse { identity: None }
                    )
                ),
            })
        );
    }

    #[test]
    fn test_identity_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let public_key_hash = vec![0; 20];
        let request = GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash: public_key_hash.clone(),
            start_after: None,
            prove: true,
        };

        let result = platform
            .query_identity_by_non_unique_public_key_hash_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                result: Some(
                    get_identity_by_non_unique_public_key_hash_response_v0::Result::Proof(_)
                ),
                metadata: Some(_),
            })
        ));
    }
}
