use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_response::DataContractEntry;
use dapi_grpc::platform::v0::get_data_contracts_response::GetDataContractsResponseV0;
use dapi_grpc::platform::v0::{get_data_contracts_response, GetDataContractsResponse, Proof};
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_data_contracts_v0(
        &self,
        state: &PlatformState,
        request: GetDataContractsRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetDataContractsRequestV0 { ids, prove } = request;
        let contract_ids = check_validation_result_with_data!(ids
            .into_iter()
            .map(|contract_id_vec| {
                Bytes32::from_vec(contract_id_vec)
                    .map(|bytes| bytes.0)
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());
        let response_data = if prove {
            let proof =
                self.drive
                    .prove_contracts(contract_ids.as_slice(), None, platform_version)?;
            GetDataContractsResponse {
                version: Some(get_data_contracts_response::Version::V0(GetDataContractsResponseV0 {
                    result: Some(get_data_contracts_response::get_data_contracts_response_v0::Result::Proof(Proof {
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
            let contracts = self.drive.get_contracts_with_fetch_info(
                contract_ids.as_slice(),
                false,
                None,
                platform_version,
            )?;

            let contracts = contracts
                .into_iter()
                .map(|(key, maybe_contract)| {
                    Ok::<DataContractEntry, ProtocolError>(DataContractEntry {
                        identifier: key.to_vec(),
                        data_contract: maybe_contract
                            .map(|contract| {
                                contract
                                    .contract
                                    .serialize_to_bytes_with_platform_version(platform_version)
                            })
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<DataContractEntry>, ProtocolError>>()?;

            GetDataContractsResponse {
                version: Some(get_data_contracts_response::Version::V0(GetDataContractsResponseV0 {
                    result: Some(get_data_contracts_response::get_data_contracts_response_v0::Result::DataContracts(get_data_contracts_response::DataContracts { data_contract_entries: contracts }
                    )),
                    metadata: Some(metadata),
                })),
            }
                .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
