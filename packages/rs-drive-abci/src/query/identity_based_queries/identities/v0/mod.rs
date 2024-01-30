use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_request::GetIdentitiesRequestV0;
use dapi_grpc::platform::v0::get_identities_response::{GetIdentitiesResponseV0, IdentityEntry};
use dapi_grpc::platform::v0::{get_identities_response, GetIdentitiesResponse, Proof};
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_identities_v0(
        &self,
        state: &PlatformState,
        get_identities_request: GetIdentitiesRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetIdentitiesRequestV0 { ids, prove } = get_identities_request;
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
        let response_data = if prove {
            let proof = self.drive.prove_full_identities(
                identity_ids.as_slice(),
                None,
                &platform_version.drive,
            )?;

            GetIdentitiesResponse {
                version: Some(get_identities_response::Version::V0(
                    GetIdentitiesResponseV0 {
                        result: Some(
                            get_identities_response::get_identities_response_v0::Result::Proof(
                                Proof {
                                    grovedb_proof: proof,
                                    quorum_hash: state.last_committed_quorum_hash().to_vec(),
                                    quorum_type,
                                    block_id_hash: state.last_committed_block_id_hash().to_vec(),
                                    signature: state.last_committed_block_signature().to_vec(),
                                    round: state.last_committed_block_round(),
                                },
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
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
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
