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
use crate::platform_types::platform_state::PlatformState;

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
        platform_state: &PlatformState,
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

            GetDataContractHistoryResponseV0 {
                result: Some(get_data_contract_history_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::platform_value;
    use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::Drive;
    use std::sync::Arc;

    fn default_request_v0() -> GetDataContractHistoryRequestV0 {
        GetDataContractHistoryRequestV0 {
            id: vec![1; 32],
            limit: Some(10),
            offset: Some(0),
            start_at_ms: 0,
            prove: false,
        }
    }

    /// Set up simple contract history with one update
    fn set_up_history(platform: &TempPlatform<MockCoreRPCLike>) -> DataContract {
        let state = platform.platform.state.load();
        let current_protocol_version = state.current_protocol_version_in_consensus();
        let platform_version = PlatformVersion::get(current_protocol_version)
            .expect("expected to get platform version");
        drop(state);
        let mut data_contract =
            get_data_contract_fixture(None, 0, current_protocol_version).data_contract_owned();
        data_contract.config_mut().set_keeps_history(true);
        data_contract.config_mut().set_readonly(false);

        platform
            .drive
            .apply_contract(
                &data_contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 10,
                    core_height: 20,
                    epoch: Default::default(),
                },
                true,
                None,
                None,
                platform_version,
            )
            .expect("To apply contract");

        let mut updated_data_contract = data_contract.clone();

        let updated_document_schema = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0
                },
                "newProp": {
                    "type": "integer",
                    "minimum": 0,
                    "position": 1
                }
            },
            "required": [
            "$createdAt"
            ],
            "additionalProperties": false
        });

        updated_data_contract
            .set_document_schema(
                "niceDocument",
                updated_document_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("to be able to set document schema");

        platform
            .drive
            .apply_contract(
                &updated_data_contract,
                BlockInfo {
                    time_ms: 2000,
                    height: 11,
                    core_height: 21,
                    epoch: Default::default(),
                },
                true,
                None,
                None,
                platform_version,
            )
            .expect("To apply contract");

        data_contract
    }

    struct TestData<'a> {
        platform: TempPlatform<MockCoreRPCLike>,
        original_data_contract: DataContract,
        version: &'a PlatformVersion,
        state: Arc<PlatformState>,
    }

    fn set_up_test<'a>() -> TestData<'a> {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let original_data_contract = set_up_history(&platform);

        // We can't return a reference to Arc (`load` method) so we clone Arc (`load_full`).
        // This is a bit slower but we don't care since we are in test environment
        let platform_state = platform.platform.state.load_full();

        TestData {
            platform,
            original_data_contract,
            version: PlatformVersion::latest(),
            state: platform_state,
        }
    }

    #[test]
    fn test_invalid_data_contract_id() {
        let (platform, state, version) = setup_platform(false);

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 8],
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_limit_overflow() {
        let (platform, state, version) = setup_platform(false);

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 32],
            limit: Some(u32::MAX),
            offset: None,
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == "limit out of bounds"
        ));
    }

    #[test]
    fn test_invalid_offset_overflow() {
        let (platform, state, version) = setup_platform(false);

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 32],
            limit: None,
            offset: Some(u32::MAX),
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            matches!(result.errors.as_slice(), [QueryError::InvalidArgument(msg)] if msg == "offset out of bounds")
        );
    }

    #[test]
    fn test_data_contract_not_found() {
        let (platform, state, version) = setup_platform(false);

        let id = vec![0; 32];

        let request = GetDataContractHistoryRequestV0 {
            id,
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: false,
        };

        let result = platform
            .query_data_contract_history_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(msg)] if msg.contains("data contract")
        ));
    }

    #[test]
    pub fn should_return_contract_history_with_no_errors_if_parameters_are_valid() {
        let TestData {
            platform,
            original_data_contract,
            version,
            state,
        } = set_up_test();

        let request = GetDataContractHistoryRequestV0 {
            id: original_data_contract.id().to_vec(),
            ..default_request_v0()
        };

        let result = platform
            .query_data_contract_history_v0(request, &state, version)
            .expect("To return result");

        let ValidationResult { errors, data } = result;

        assert!(
            errors.is_empty(),
            "expect no errors to be returned from the query"
        );

        let response = data.expect("expect data to be returned from the query");

        let GetDataContractHistoryResponseV0 {
            result:
                Some(get_data_contract_history_response_v0::Result::DataContractHistory(
                    data_contract_history,
                )),
            metadata: Some(_),
        } = response
        else {
            panic!("expect result to be DataContractHistory");
        };

        let mut history_entries = data_contract_history.data_contract_entries;
        assert_eq!(history_entries.len(), 2);

        let second_entry = history_entries.pop().unwrap();
        let first_entry = history_entries.pop().unwrap();

        assert_eq!(first_entry.date, 1000);
        let first_entry_data_contract = first_entry.value;
        let first_data_contract_update =
            DataContract::versioned_deserialize(&first_entry_data_contract, true, version)
                .expect("To decode data contract");
        assert_eq!(first_data_contract_update, original_data_contract);

        assert_eq!(second_entry.date, 2000);

        let second_entry_data_contract = second_entry.value;

        let second_data_contract_update =
            DataContract::versioned_deserialize(&second_entry_data_contract, true, version)
                .expect("To decode data contract");

        let updated_doc = second_data_contract_update
            .document_type_for_name("niceDocument")
            .expect("should return document type");

        assert!(
            updated_doc.properties().contains_key("newProp"),
            "expect data contract to have newProp field",
        );
    }

    #[test]
    pub fn should_return_contract_history_proofs_with_no_errors_if_parameters_are_valid() {
        let TestData {
            platform,
            original_data_contract,
            version,
            state,
        } = set_up_test();

        let request_v0 = GetDataContractHistoryRequestV0 {
            id: original_data_contract.id().to_vec(),
            prove: true,
            ..default_request_v0()
        };

        let start_at_ms = request_v0.start_at_ms;

        let result = platform
            .query_data_contract_history_v0(request_v0, &state, version)
            .expect("To return result");

        let ValidationResult { errors, data } = result;

        assert!(
            errors.is_empty(),
            "expect no errors to be returned from the query"
        );

        let response = data.expect("expect data to be returned from the query");

        let GetDataContractHistoryResponseV0 {
            result: Some(get_data_contract_history_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        } = response
        else {
            panic!("expect result to be Proof");
        };

        // Check that the proof has correct values inside
        let (_root_hash, contract_history) = Drive::verify_contract_history(
            &proof.grovedb_proof,
            original_data_contract.id().to_buffer(),
            start_at_ms,
            Some(10),
            None,
            version,
        )
        .expect("To verify contract history");

        let mut history_entries = contract_history.expect("history to exist");

        assert_eq!(history_entries.len(), 2);

        // Taking entries by date
        let first_data_contract_update =
            history_entries.remove(&1000).expect("first entry to exist");
        let second_data_contract_update = history_entries
            .remove(&2000)
            .expect("second entry to exist");

        assert_eq!(first_data_contract_update, original_data_contract);

        let updated_doc = second_data_contract_update
            .document_type_for_name("niceDocument")
            .expect("To have niceDocument document");

        assert!(
            updated_doc.properties().contains_key("newProp"),
            "expect data contract to have newProp field",
        );
    }

    #[test]
    fn test_data_contract_history_absence_proof() {
        let (platform, state, version) = setup_platform(false);

        let request = GetDataContractHistoryRequestV0 {
            id: vec![0; 32],
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: true,
        };

        let result = platform
            .query_data_contract_history_v0(request, &state, version)
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
