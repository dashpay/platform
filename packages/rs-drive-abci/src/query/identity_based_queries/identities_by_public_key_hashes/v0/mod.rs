use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{
    get_identities_by_public_key_hashes_response, get_identity_response,
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse,
    GetIdentityRequest, GetIdentityResponse, Proof, ResponseMetadata,
};
use dpp::platform_value::Bytes20;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_identities_by_public_key_hashes_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetIdentitiesByPublicKeyHashesRequest {
            public_key_hashes,
            prove,
        } = check_validation_result_with_data!(GetIdentitiesByPublicKeyHashesRequest::decode(
            query_data
        ));
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
        let response_data = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_full_identities_by_unique_public_key_hashes(
                    &public_key_hashes,
                    None,
                    platform_version,
                ));
            GetIdentitiesByPublicKeyHashesResponse {
                result: Some(get_identities_by_public_key_hashes_response::Result::Proof(
                    Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_block_id_hash().to_vec(),
                        signature: state.last_block_signature().to_vec(),
                        round: state.last_block_round(),
                    },
                )),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        } else {
            //todo: fix this so we return optionals
            let identities = check_validation_result_with_data!(self
                .drive
                .fetch_full_identities_by_unique_public_key_hashes(
                    public_key_hashes.as_slice(),
                    None,
                    platform_version,
                ));
            let identities = check_validation_result_with_data!(identities
                .into_values()
                .filter_map(|maybe_identity| Some(maybe_identity?.serialize_consume_to_bytes()))
                .collect::<Result<Vec<Vec<u8>>, ProtocolError>>());
            GetIdentitiesByPublicKeyHashesResponse {
                result: Some(
                    get_identities_by_public_key_hashes_response::Result::Identities(
                        get_identities_by_public_key_hashes_response::Identities { identities },
                    ),
                ),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
