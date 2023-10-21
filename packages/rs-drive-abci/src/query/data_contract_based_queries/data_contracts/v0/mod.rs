use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_response::DataContractEntry;
use dapi_grpc::platform::v0::{
    get_data_contracts_response, GetDataContractsRequest, GetDataContractsResponse, Proof,
};
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
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetDataContractsRequest { ids, prove } =
            check_validation_result_with_data!(GetDataContractsRequest::decode(query_data));
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
            let proof = check_validation_result_with_data!(self.drive.prove_contracts(
                contract_ids.as_slice(),
                None,
                platform_version
            ));
            GetDataContractsResponse {
                metadata: Some(metadata),
                result: Some(get_data_contracts_response::Result::Proof(Proof {
                    grovedb_proof: proof,
                    quorum_hash: state.last_quorum_hash().to_vec(),
                    quorum_type,
                    block_id_hash: state.last_block_id_hash().to_vec(),
                    signature: state.last_block_signature().to_vec(),
                    round: state.last_block_round(),
                })),
            }
            .encode_to_vec()
        } else {
            let contracts =
                check_validation_result_with_data!(self.drive.get_contracts_with_fetch_info(
                    contract_ids.as_slice(),
                    false,
                    None,
                    platform_version
                ));

            let contracts = check_validation_result_with_data!(contracts
                .into_iter()
                .map(
                    |(key, maybe_contract)| Ok::<DataContractEntry, ProtocolError>(
                        get_data_contracts_response::DataContractEntry {
                            key: key.to_vec(),
                            value: maybe_contract
                                .map(|contract| Ok::<
                                    get_data_contracts_response::DataContractValue,
                                    ProtocolError,
                                >(
                                    get_data_contracts_response::DataContractValue {
                                        value: contract
                                            .contract
                                            .serialize_to_bytes_with_platform_version(
                                                platform_version
                                            )?
                                    }
                                ))
                                .transpose()?,
                        }
                    )
                )
                .collect());
            GetDataContractsResponse {
                result: Some(get_data_contracts_response::Result::DataContracts(
                    get_data_contracts_response::DataContracts {
                        data_contract_entries: contracts,
                    },
                )),
                metadata: Some(metadata),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
