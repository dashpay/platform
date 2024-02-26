use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
use dapi_grpc::platform::v0::get_data_contract_history_response::{get_data_contract_history_response_v0, GetDataContractHistoryResponseV0};
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use dapi_grpc::platform::v0::get_data_contract_history_response::get_data_contract_history_response_v0::DataContractHistoryEntry;

impl<C> Platform<C> {
    pub(super) fn query_data_contract_history_v0(
        &self,
        GetDataContractHistoryRequestV0 {
            id,
            limit,
            offset,
            start_at_ms,
            prove,
        }: GetDataContractHistoryRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractHistoryResponseV0>, Error> {
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

        let response = if prove {
            let proof = self.drive.prove_contract_history(
                contract_id.to_buffer(),
                None,
                start_at_ms,
                limit,
                offset,
                platform_version,
            )?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetDataContractHistoryResponseV0 {
                result: Some(get_data_contract_history_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
            }
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

            GetDataContractHistoryResponseV0 {
                result: Some(
                    get_data_contract_history_response_v0::Result::DataContractHistory(
                        get_data_contract_history_response_v0::DataContractHistory {
                            data_contract_entries: contract_historical_entries,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0()),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};

    #[test]
    fn test_invalid_data_contract_id() {
        let (platform, version) = setup_platform();

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 8],
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_limit_overflow() {
        let (platform, version) = setup_platform();

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 32],
            limit: Some(u32::MAX),
            offset: None,
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == "limit out of bounds"
        ));
    }

    #[test]
    fn test_invalid_offset_overflow() {
        let (platform, version) = setup_platform();

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 32],
            limit: None,
            offset: Some(u32::MAX),
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, version)
            .expect("expected query to succeed");

        assert!(
            matches!(result.errors.as_slice(), [QueryError::InvalidArgument(msg)] if msg == "offset out of bounds")
        );
    }

    #[test]
    fn test_data_contract_not_found() {
        let (platform, version) = setup_platform();

        let id = vec![0; 32];

        let request = GetDataContractHistoryRequestV0 {
            id,
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(msg)] if msg.contains("data contract")
        ));
    }

    #[test]
    fn test_data_contract_history_absence_proof() {
        let (platform, version) = setup_platform();

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 32],
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: true,
        };

        let result = platform
            .query_data_contract_history_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDataContractHistoryResponseV0 {
                result: Some(get_data_contract_history_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
