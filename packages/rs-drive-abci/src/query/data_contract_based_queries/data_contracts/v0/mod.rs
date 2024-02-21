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
use dapi_grpc::Message;
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};

impl<C> Platform<C> {
    pub(super) fn query_data_contracts_v0(
        &self,
        GetDataContractsRequestV0 { ids, prove }: GetDataContractsRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractsResponse>, Error> {
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

        let response = if prove {
            let proof =
                self.drive
                    .prove_contracts(contract_ids.as_slice(), None, platform_version)?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetDataContractsResponse {
                version: Some(get_data_contracts_response::Version::V0(GetDataContractsResponseV0 {
                    result: Some(get_data_contracts_response::get_data_contracts_response_v0::Result::Proof(proof)),
                    metadata: Some(metadata),
                })),
            }
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
                    metadata: Some(self.response_metadata_v0()),
                })),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
