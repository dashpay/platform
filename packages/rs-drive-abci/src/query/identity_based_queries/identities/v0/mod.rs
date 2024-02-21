use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_request::GetIdentitiesRequestV0;
use dapi_grpc::platform::v0::get_identities_response::{GetIdentitiesResponseV0, IdentityEntry};
use dapi_grpc::platform::v0::{get_identities_response, GetIdentitiesResponse};
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};

impl<C> Platform<C> {
    pub(super) fn query_identities_v0(
        &self,
        GetIdentitiesRequestV0 { ids, prove }: GetIdentitiesRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesResponse>, Error> {
        let identity_ids = check_validation_result_with_data!(ids
            .into_iter()
            .map(|identity_id_vec| {
                Bytes32::from_vec(identity_id_vec)
                    .map(|bytes| bytes.0)
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let response = if prove {
            let proof = self.drive.prove_full_identities(
                identity_ids.as_slice(),
                None,
                &platform_version.drive,
            )?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetIdentitiesResponse {
                version: Some(get_identities_response::Version::V0(
                    GetIdentitiesResponseV0 {
                        result: Some(
                            get_identities_response::get_identities_response_v0::Result::Proof(
                                proof,
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
        } else {
            let identities = self.drive.fetch_full_identities(
                identity_ids.as_slice(),
                None,
                platform_version,
            )?;

            let identities = identities
                .into_iter()
                .map(|(key, maybe_identity)| {
                    Ok::<IdentityEntry, ProtocolError>(IdentityEntry {
                        key: key.to_vec(),
                        value: maybe_identity
                            .map(|identity| {
                                Ok::<get_identities_response::IdentityValue, ProtocolError>(
                                    get_identities_response::IdentityValue {
                                        value: identity.serialize_consume_to_bytes()?,
                                    },
                                )
                            })
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<IdentityEntry>, ProtocolError>>()?;

            GetIdentitiesResponse {
                version: Some(get_identities_response::Version::V0(
                    GetIdentitiesResponseV0 {
                        result: Some(
                            get_identities_response::get_identities_response_v0::Result::Identities(
                                get_identities_response::Identities {
                                    identity_entries: identities,
                                },
                            ),
                        ),
                        metadata: Some(self.response_metadata_v0()),
                    },
                )),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
