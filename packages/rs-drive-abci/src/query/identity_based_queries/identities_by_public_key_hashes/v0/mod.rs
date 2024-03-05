use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_request::GetIdentitiesByPublicKeyHashesRequestV0;
use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_response;
use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_response::{
    get_identities_by_public_key_hashes_response_v0, GetIdentitiesByPublicKeyHashesResponseV0,
    PublicKeyHashIdentityEntry,
};
use dpp::platform_value::Bytes20;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};

impl<C> Platform<C> {
    pub(super) fn query_identities_by_public_key_hashes_v0(
        &self,
        GetIdentitiesByPublicKeyHashesRequestV0 {
            public_key_hashes,
            prove,
        }: GetIdentitiesByPublicKeyHashesRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesByPublicKeyHashesResponseV0>, Error> {
        let public_key_hashes = check_validation_result_with_data!(public_key_hashes
            .into_iter()
            .map(|pub_key_hash_vec| {
                Bytes20::from_vec(pub_key_hash_vec)
                    .map(|bytes| bytes.0)
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "public key hash must be 20 bytes long".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<[u8; 20]>, QueryError>>());

        let response = if prove {
            let proof = self
                .drive
                .prove_full_identities_by_unique_public_key_hashes(
                    &public_key_hashes,
                    None,
                    platform_version,
                )?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetIdentitiesByPublicKeyHashesResponseV0 {
                result: Some(get_identities_by_public_key_hashes_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
            }
        } else {
            let identities = self
                .drive
                .fetch_full_identities_by_unique_public_key_hashes(
                    public_key_hashes.as_slice(),
                    None,
                    platform_version,
                )?;
            let identities = identities
                .into_iter()
                .map(|(hash, maybe_identity)| {
                    Ok(PublicKeyHashIdentityEntry {
                        public_key_hash: hash.to_vec(),
                        value: maybe_identity
                            .map(|identity| identity.serialize_consume_to_bytes())
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<PublicKeyHashIdentityEntry>, ProtocolError>>()?;

            GetIdentitiesByPublicKeyHashesResponseV0 {
                metadata: Some(self.response_metadata_v0()),
                result: Some(
                    get_identities_by_public_key_hashes_response_v0::Result::Identities(
                        get_identities_by_public_key_hashes_response::IdentitiesByPublicKeyHashes {
                            identity_entries: identities,
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

    #[test]
    fn test_invalid_public_key_hash() {
        let (platform, version) = setup_platform();

        let request = GetIdentitiesByPublicKeyHashesRequestV0 {
            public_key_hashes: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_identities_by_public_key_hashes_v0(request, version)
            .expect("should query identities by public key hashes");

        assert!(
            matches!(result.errors.as_slice(), [QueryError::InvalidArgument(msg)] if msg == "public key hash must be 20 bytes long")
        );
    }

    #[test]
    fn test_identities_not_found() {
        let (platform, version) = setup_platform();

        let request = GetIdentitiesByPublicKeyHashesRequestV0 {
            public_key_hashes: vec![vec![0; 20]],
            prove: false,
        };

        let result = platform
            .query_identities_by_public_key_hashes_v0(request, version)
            .expect("expected query to succeed");

        assert!(
            matches!(result.data, Some(GetIdentitiesByPublicKeyHashesResponseV0 {
            result: Some(get_identities_by_public_key_hashes_response_v0::Result::Identities(identites)),
            metadata: Some(_)
        }) if identites.identity_entries.len() == 1 && identites.identity_entries[0].value.is_none())
        );
    }

    #[test]
    fn test_identities_absence_proof() {
        let (platform, version) = setup_platform();

        let request = GetIdentitiesByPublicKeyHashesRequestV0 {
            public_key_hashes: vec![vec![0; 20]],
            prove: true,
        };

        let result = platform
            .query_identities_by_public_key_hashes_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentitiesByPublicKeyHashesResponseV0 {
                result: Some(get_identities_by_public_key_hashes_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ));
    }
}
