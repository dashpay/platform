use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::{
    get_data_contract_response, GetDataContractRequest, GetDataContractResponse,
    GetIdentityBalanceResponse, GetIdentityRequest, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_data_contract_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetDataContractRequest { id, prove } =
            check_validation_result_with_data!(GetDataContractRequest::decode(query_data));
        let contract_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));
        let response_data = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_contract(
                contract_id.into_buffer(),
                None,
                platform_version
            ));

            GetDataContractResponse {
                result: Some(get_data_contract_response::Result::Proof(Proof {
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
            let maybe_data_contract = check_validation_result_with_data!(self
                .drive
                .fetch_contract(
                    contract_id.into_buffer(),
                    None,
                    None,
                    None,
                    platform_version
                )
                .unwrap());

            let data_contract = check_validation_result_with_data!(maybe_data_contract
                .ok_or_else(|| {
                    QueryError::NotFound(format!("data contract {} not found", contract_id))
                })
                .and_then(|data_contract| data_contract
                    .contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .map_err(QueryError::Protocol)));

            GetDataContractResponse {
                result: Some(get_data_contract_response::Result::DataContract(
                    data_contract,
                )),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
