use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0;
use dapi_grpc::platform::v0::get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0;
use dapi_grpc::platform::v0::{
    get_identity_contract_nonce_response, GetIdentityContractNonceResponse, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_identity_contract_nonce_v0(
        &self,
        state: &PlatformState,
        request: GetIdentityContractNonceRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetIdentityContractNonceRequestV0 {
            identity_id,
            contract_id,
            prove,
        } = request;
        let identity_id = check_validation_result_with_data!(Identifier::from_vec(identity_id)
            .map(|bytes| bytes.0)
            .map_err(|_| QueryError::InvalidArgument(
                "identity id must be 32 bytes long".to_string()
            )));
        let contract_id = check_validation_result_with_data!(Identifier::from_vec(contract_id)
            .map(|bytes| bytes.0)
            .map_err(|_| QueryError::InvalidArgument(
                "contract id must be 32 bytes long".to_string()
            )));
        let response_data = if prove {
            let proof = self.drive.prove_identity_contract_nonce(
                identity_id.0,
                contract_id.0,
                None,
                &platform_version.drive,
            )?;

            GetIdentityContractNonceResponse {
                version: Some(get_identity_contract_nonce_response::Version::V0(GetIdentityContractNonceResponseV0 {
                    result: Some(get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_committed_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_committed_block_id_hash().to_vec(),
                        signature: state.last_committed_block_signature().to_vec(),
                        round: state.last_committed_block_round(),
                    })),
                    metadata: Some(metadata),
                })),
            }
                .encode_to_vec()
        } else {
            let maybe_identity = self.drive.fetch_identity_contract_nonce(
                identity_id.0,
                contract_id.0,
                true,
                None,
                platform_version,
            )?;

            // default here is 0;
            let identity_contract_nonce = maybe_identity.unwrap_or_default();

            GetIdentityContractNonceResponse {
                version: Some(get_identity_contract_nonce_response::Version::V0(GetIdentityContractNonceResponseV0 {
                    metadata: Some(metadata),
                    result: Some(get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::IdentityContractNonce(identity_contract_nonce)),
                })),
            }
                .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
