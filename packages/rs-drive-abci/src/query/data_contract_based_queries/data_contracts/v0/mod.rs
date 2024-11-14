use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_response;
use dapi_grpc::platform::v0::get_data_contracts_response::GetDataContractsResponseV0;
use dapi_grpc::platform::v0::get_data_contracts_response::{
    get_data_contracts_response_v0, DataContractEntry,
};
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};

impl<C> Platform<C> {
    pub(super) fn query_data_contracts_v0(
        &self,
        GetDataContractsRequestV0 { ids, prove }: GetDataContractsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractsResponseV0>, Error> {
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

            GetDataContractsResponseV0 {
                result: Some(get_data_contracts_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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

            GetDataContractsResponseV0 {
                result: Some(get_data_contracts_response_v0::Result::DataContracts(
                    get_data_contracts_response::DataContracts {
                        data_contract_entries: contracts,
                    },
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_data_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDataContractsRequestV0 {
            ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_data_contracts_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_data_contracts_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];
        let request = GetDataContractsRequestV0 {
            ids: vec![id.clone()],
            prove: false,
        };

        let result = platform
            .query_data_contracts_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDataContractsResponseV0 {
                result: Some(get_data_contracts_response_v0::Result::DataContracts(contracts)),
                metadata: Some(_),
            }) if contracts.data_contract_entries.len() == 1 && contracts.data_contract_entries[0].data_contract.is_none()
        ));
    }

    #[test]
    fn test_data_contracts_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];
        let request = GetDataContractsRequestV0 {
            ids: vec![id.clone()],
            prove: true,
        };

        let result = platform
            .query_data_contracts_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDataContractsResponseV0 {
                result: Some(get_data_contracts_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
