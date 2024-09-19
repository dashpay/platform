use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_response::{
    get_identity_by_public_key_hash_response_v0, GetIdentityByPublicKeyHashResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Bytes20;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_by_public_key_hash_v0(
        &self,
        GetIdentityByPublicKeyHashRequestV0 {
            public_key_hash,
            prove,
        }: GetIdentityByPublicKeyHashRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityByPublicKeyHashResponseV0>, Error> {
        let public_key_hash =
            check_validation_result_with_data!(Bytes20::from_vec(public_key_hash)
                .map(|bytes| bytes.0)
                .map_err(|_| QueryError::InvalidArgument(
                    "public key hash must be 20 bytes long".to_string()
                )));

        let response = if prove {
            let proof = self.drive.prove_full_identity_by_unique_public_key_hash(
                public_key_hash,
                None,
                platform_version,
            )?;

            GetIdentityByPublicKeyHashResponseV0 {
                result: Some(get_identity_by_public_key_hash_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let maybe_identity = self.drive.fetch_full_identity_by_unique_public_key_hash(
                public_key_hash,
                None,
                platform_version,
            )?;

            let identity = check_validation_result_with_data!(maybe_identity.ok_or_else(|| {
                QueryError::NotFound(format!(
                    "identity for public key hash {} not found",
                    hex::encode(public_key_hash)
                ))
            }));

            let serialized_identity = identity
                .serialize_consume_to_bytes()
                .map_err(Error::Protocol)?;

            GetIdentityByPublicKeyHashResponseV0 {
                metadata: Some(self.response_metadata_v0(platform_state)),
                result: Some(
                    get_identity_by_public_key_hash_response_v0::Result::Identity(
                        serialized_identity,
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
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_public_key_hash() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityByPublicKeyHashRequestV0 {
            public_key_hash: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_identity_by_public_key_hash_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == &"public key hash must be 20 bytes long".to_string()
        ));
    }

    #[test]
    fn test_identity_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let public_key_hash = vec![0; 20];
        let request = GetIdentityByPublicKeyHashRequestV0 {
            public_key_hash: public_key_hash.clone(),
            prove: false,
        };

        let result = platform
            .query_identity_by_public_key_hash_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(msg)] if msg.contains("identity for public key hash")
        ))
    }

    #[test]
    fn test_identity_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let public_key_hash = vec![0; 20];
        let request = GetIdentityByPublicKeyHashRequestV0 {
            public_key_hash: public_key_hash.clone(),
            prove: true,
        };

        let result = platform
            .query_identity_by_public_key_hash_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentityByPublicKeyHashResponseV0 {
                result: Some(get_identity_by_public_key_hash_response_v0::Result::Proof(
                    _
                )),
                metadata: Some(_),
            })
        ));
    }
}
