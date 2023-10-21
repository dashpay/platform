use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{
    get_identity_response, GetIdentityRequest, GetIdentityResponse, Proof, ResponseMetadata,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_identity_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetIdentityRequest { id, prove } =
            check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
        let identity_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));
        let response_data = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_full_identity(
                identity_id.into_buffer(),
                None,
                &platform_version.drive
            ));

            GetIdentityResponse {
                result: Some(get_identity_response::Result::Proof(Proof {
                    grovedb_proof: proof,
                    quorum_hash: state.last_quorum_hash().to_vec(),
                    quorum_type,
                    block_id_hash: state.last_block_id_hash().to_vec(),
                    signature: state.last_block_signature().to_vec(),
                    round: state.last_block_round(),
                })),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        } else {
            let maybe_identity = check_validation_result_with_data!(self
                .drive
                .fetch_full_identity(identity_id.into_buffer(), None, platform_version)
                .map_err(QueryError::Drive));

            let identity = check_validation_result_with_data!(maybe_identity
                .ok_or_else(|| {
                    QueryError::NotFound(format!("identity {} not found", identity_id))
                })
                .and_then(|identity| identity
                    .serialize_consume_to_bytes()
                    .map_err(QueryError::Protocol)));

            GetIdentityResponse {
                result: Some(get_identity_response::Result::Identity(identity)),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
