use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
use dapi_grpc::platform::v0::get_data_contract_history_response::GetDataContractHistoryResponseV0;
use dapi_grpc::platform::v0::{
    get_data_contract_history_response, GetDataContractHistoryResponse, Proof,
};
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use prost::Message;
use dapi_grpc::platform::v0::get_data_contract_history_response::get_data_contract_history_response_v0::DataContractHistoryEntry;

impl<C> Platform<C> {
    pub(super) fn query_data_contract_history_v0(
        &self,
        state: &PlatformState,
        request: GetDataContractHistoryRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetDataContractHistoryRequestV0 {
            id,
            limit,
            offset,
            start_at_ms,
            prove,
        } = request;
        let contract_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = check_validation_result_with_data!(limit
            .map(|limit| {
                u16::try_from(limit)
                    .map_err(|_| QueryError::InvalidArgument("limit out of bounds".to_string()))
            })
            .transpose());
        let offset = check_validation_result_with_data!(offset
            .map(|offset| {
                u16::try_from(offset)
                    .map_err(|_| QueryError::InvalidArgument("offset out of bounds".to_string()))
            })
            .transpose());

        let response_data = if prove {
            let proof = self.drive.prove_contract_history(
                contract_id.to_buffer(),
                None,
                start_at_ms,
                limit,
                offset,
                platform_version,
            )?;
            GetDataContractHistoryResponse {
                version: Some(get_data_contract_history_response::Version::V0(GetDataContractHistoryResponseV0 {
                    result: Some(get_data_contract_history_response::get_data_contract_history_response_v0::Result::Proof(Proof {
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
            let contracts = self.drive.fetch_contract_with_history(
                contract_id.to_buffer(),
                None,
                start_at_ms,
                limit,
                offset,
                platform_version,
            )?;

            if contracts.is_empty() {
                return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                    format!("data contract {} history not found", contract_id),
                )));
            }

            let contract_historical_entries: Vec<DataContractHistoryEntry> = contracts
                .into_iter()
                .map(|(date_in_seconds, data_contract)| {
                    Ok::<DataContractHistoryEntry, ProtocolError>(DataContractHistoryEntry {
                        date: date_in_seconds,
                        value: data_contract
                            .serialize_to_bytes_with_platform_version(platform_version)?,
                    })
                })
                .collect::<Result<Vec<DataContractHistoryEntry>, ProtocolError>>()?;

            GetDataContractHistoryResponse {
                version: Some(get_data_contract_history_response::Version::V0(GetDataContractHistoryResponseV0 {
                    result: Some(get_data_contract_history_response::get_data_contract_history_response_v0::Result::DataContractHistory(
                        get_data_contract_history_response::get_data_contract_history_response_v0::DataContractHistory {
                            data_contract_entries: contract_historical_entries,
                        }
                    )),
                    metadata: Some(metadata),
                })),
            }
                .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
