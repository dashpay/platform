use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{
    get_identity_by_public_key_hashes_response, GetIdentityByPublicKeyHashesRequest,
    GetIdentityByPublicKeyHashesResponse, GetIdentityRequest, GetIdentityResponse, Proof,
    ResponseMetadata,
};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Bytes20;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_identity_by_public_key_hash_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetIdentityByPublicKeyHashesRequest {
            public_key_hash,
            prove,
        } = check_validation_result_with_data!(GetIdentityByPublicKeyHashesRequest::decode(
            query_data
        ));
        let public_key_hash =
            check_validation_result_with_data!(Bytes20::from_vec(public_key_hash)
                .map(|bytes| bytes.0)
                .map_err(|_| QueryError::InvalidArgument(
                    "public key hash must be 20 bytes long".to_string()
                )));
        let response_data = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_full_identity_by_unique_public_key_hash(
                    public_key_hash,
                    None,
                    platform_version
                ));

            GetIdentityByPublicKeyHashesResponse {
                metadata: Some(metadata),
                result: Some(get_identity_by_public_key_hashes_response::Result::Proof(
                    Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_block_id_hash().to_vec(),
                        signature: state.last_block_signature().to_vec(),
                        round: state.last_block_round(),
                    },
                )),
            }
            .encode_to_vec()
        } else {
            let maybe_identity = check_validation_result_with_data!(self
                .drive
                .fetch_full_identity_by_unique_public_key_hash(
                    public_key_hash,
                    None,
                    platform_version
                ));
            let serialized_identity = check_validation_result_with_data!(maybe_identity
                .ok_or_else(|| {
                    QueryError::NotFound(format!(
                        "identity {} not found",
                        hex::encode(public_key_hash)
                    ))
                })
                .and_then(|identity| identity
                    .serialize_consume_to_bytes()
                    .map_err(QueryError::Protocol)));

            GetIdentityByPublicKeyHashesResponse {
                metadata: Some(metadata),
                result: Some(
                    get_identity_by_public_key_hashes_response::Result::Identity(
                        serialized_identity,
                    ),
                ),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
