use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryError;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::BalanceAndRevision;
use dapi_grpc::platform::v0::{
    get_identity_balance_and_revision_response, get_identity_balance_response,
    GetIdentityBalanceAndRevisionResponse, GetIdentityBalanceResponse, GetIdentityRequest, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_balance_and_revision_v0(
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
            let proof =
                check_validation_result_with_data!(self.drive.prove_identity_balance_and_revision(
                    identity_id.into_buffer(),
                    None,
                    &platform_version.drive
                ));
            GetIdentityBalanceResponse {
                result: Some(get_identity_balance_response::Result::Proof(Proof {
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
            let balance = check_validation_result_with_data!(self.drive.fetch_identity_balance(
                identity_id.into_buffer(),
                None,
                platform_version
            ));
            let revision = check_validation_result_with_data!(self.drive.fetch_identity_revision(
                identity_id.into_buffer(),
                true,
                None,
                platform_version
            ));
            GetIdentityBalanceAndRevisionResponse {
                result: Some(
                    get_identity_balance_and_revision_response::Result::BalanceAndRevision(
                        BalanceAndRevision { balance, revision },
                    ),
                ),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
