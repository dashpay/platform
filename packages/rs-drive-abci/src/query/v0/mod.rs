use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::query::QueryValidationResult;
use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};

use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying
    pub(super) fn query_v0(
        &self,
        query_path: &str,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let state = self.state.read().unwrap();
        match query_path {
            "/identity" => self.query_identity(&state, query_data, platform_version),
            "/identities" => self.query_identities(&state, query_data, platform_version),
            "/identity/balance" => self.query_balance(&state, query_data, platform_version),
            "/identity/balanceAndRevision" => {
                self.query_balance_and_revision(&state, query_data, platform_version)
            }
            "/identity/keys" => self.query_keys(&state, query_data, platform_version),
            "/identity/by-public-key-hash" => {
                self.query_identity_by_public_key_hash(&state, query_data, platform_version)
            }
            "/identities/by-public-key-hash" => {
                self.query_identities_by_public_key_hashes(&state, query_data, platform_version)
            }
            "/dataContract" => self.query_data_contract(&state, query_data, platform_version),
            "/dataContracts" => self.query_data_contracts(&state, query_data, platform_version),
            "/dataContractHistory" => {
                self.query_data_contract_history(&state, query_data, platform_version)
            }
            "/documents" | "/dataContract/documents" => {
                self.query_documents(&state, query_data, platform_version)
            }
            "/proofs" => self.query_proofs(&state, query_data, platform_version),
            other => Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!("query path '{}' is not supported", other)),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    pub mod query_data_contract_history {
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
        use dapi_grpc::platform::v0::{
            get_data_contract_history_request, get_data_contract_history_response,
            GetDataContractHistoryRequest, GetDataContractHistoryResponse,
        };
        use dpp::block::block_info::BlockInfo;

        use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::config::v0::DataContractConfigSettersV0;

        use crate::error::query::QueryError;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::schema::DataContractSchemaMethodsV0;
        use dpp::data_contract::DataContract;
        use dpp::platform_value::platform_value;
        use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use dpp::validation::ValidationResult;
        use dpp::version::PlatformVersion;
        use drive::drive::Drive;
        use prost::Message;
        use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
        use dapi_grpc::platform::v0::get_data_contract_history_response::{get_data_contract_history_response_v0, GetDataContractHistoryResponseV0};

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
            let state = platform.platform.state.read().unwrap();
            let current_protocol_version = state.current_protocol_version_in_consensus();
            let platform_version = PlatformVersion::get(current_protocol_version)
                .expect("expected to get platform version");
            drop(state);
            let mut data_contract =
                get_data_contract_fixture(None, current_protocol_version).data_contract_owned();
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
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
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

        struct TestData {
            platform: TempPlatform<MockCoreRPCLike>,
            original_data_contract: DataContract,
        }

        fn set_up_test() -> TestData {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let original_data_contract = set_up_history(&platform);

            TestData {
                platform,
                original_data_contract,
            }
        }

        #[test]
        pub fn should_return_contract_history_with_no_errors_if_parameters_are_valid() {
            let platform_version = PlatformVersion::latest();

            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                version: Some(get_data_contract_history_request::Version::V0(
                    GetDataContractHistoryRequestV0 {
                        id: original_data_contract.id().to_vec(),
                        ..default_request_v0()
                    },
                )),
            };

            let request_data = request.encode_to_vec();

            let result = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .expect("To return result");

            let ValidationResult { errors, data } = result;

            assert!(
                errors.is_empty(),
                "expect no errors to be returned from the query"
            );

            let data = data.expect("expect data to be returned from the query");
            let _data_ref = data.as_slice();

            let response = GetDataContractHistoryResponse::decode(data.as_slice())
                .expect("To decode response");

            let GetDataContractHistoryResponse { version } = response;

            let get_data_contract_history_response::Version::V0(GetDataContractHistoryResponseV0 {
                result,
                metadata: _,
            }) = version.expect("expected a versioned response");

            let res = result.expect("expect result to be returned from the query");

            let contract_history = match res {
                get_data_contract_history_response_v0::Result::DataContractHistory(
                    data_contract_history,
                ) => data_contract_history,
                get_data_contract_history_response_v0::Result::Proof(_) => {
                    panic!("expect result to be DataContractHistory");
                }
            };

            let mut history_entries = contract_history.data_contract_entries;
            assert_eq!(history_entries.len(), 2);

            let second_entry = history_entries.pop().unwrap();
            let first_entry = history_entries.pop().unwrap();

            assert_eq!(first_entry.date, 1000);
            let first_entry_data_contract = first_entry.value;
            let first_data_contract_update = DataContract::versioned_deserialize(
                &first_entry_data_contract,
                true,
                platform_version,
            )
            .expect("To decode data contract");
            assert_eq!(first_data_contract_update, original_data_contract);

            assert_eq!(second_entry.date, 2000);

            let second_entry_data_contract = second_entry.value;

            let second_data_contract_update = DataContract::versioned_deserialize(
                &second_entry_data_contract,
                true,
                platform_version,
            )
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
            let platform_version = PlatformVersion::latest();
            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request_v0 = GetDataContractHistoryRequestV0 {
                id: original_data_contract.id().to_vec(),
                prove: true,
                ..default_request_v0()
            };

            let start_at_ms = request_v0.start_at_ms;

            let request = GetDataContractHistoryRequest {
                version: Some(get_data_contract_history_request::Version::V0(request_v0)),
            };

            let request_data = request.encode_to_vec();

            let result = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .expect("To return result");

            let ValidationResult { errors, data } = result;

            assert!(
                errors.is_empty(),
                "expect no errors to be returned from the query"
            );

            let data = data.expect("expect data to be returned from the query");
            let _data_ref = data.as_slice();

            let response = GetDataContractHistoryResponse::decode(data.as_slice())
                .expect("To decode response");

            let GetDataContractHistoryResponse { version } = response;

            let get_data_contract_history_response::Version::V0(GetDataContractHistoryResponseV0 {
                result,
                metadata: _,
            }) = version.expect("expected a versioned response");

            let res = result.expect("expect result to be returned from the query");

            let contract_proof = match res {
                get_data_contract_history_response_v0::Result::DataContractHistory(_) => {
                    panic!("expect result to be Proof");
                }
                get_data_contract_history_response_v0::Result::Proof(proof) => proof,
            };

            // Check that the proof has correct values inside
            let (_root_hash, contract_history) = Drive::verify_contract_history(
                &contract_proof.grovedb_proof,
                original_data_contract.id().to_buffer(),
                start_at_ms,
                Some(10),
                Some(0),
                platform_version,
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
        pub fn should_return_error_when_limit_is_larger_than_u16() {
            let platform_version = PlatformVersion::latest();
            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                version: Some(get_data_contract_history_request::Version::V0(
                    GetDataContractHistoryRequestV0 {
                        id: original_data_contract.id().to_vec(),
                        limit: Some(100000),
                        ..default_request_v0()
                    },
                )),
            };

            let request_data = request.encode_to_vec();

            let validation_result = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .expect("To return validation result with an error");

            assert!(!validation_result.is_valid());

            let error = validation_result.errors.first().expect("expect error to exist");

            assert!(matches!(error, QueryError::InvalidArgument(_)));

            assert_eq!(
                error.to_string(),
                "invalid argument error: limit out of bounds"
            );
        }

        #[test]
        pub fn should_return_error_when_offset_is_larger_than_u16() {
            let platform_version = PlatformVersion::latest();

            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                version: Some(get_data_contract_history_request::Version::V0(
                    GetDataContractHistoryRequestV0 {
                        id: original_data_contract.id().to_vec(),
                        offset: Some(100000),
                        ..default_request_v0()
                    },
                )),
            };

            let request_data = request.encode_to_vec();

            let validation_result = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .expect("To return validation result with an error");

            assert!(!validation_result.is_valid());

            let error = validation_result.errors.first().expect("expect error to exist");

            assert!(matches!(error, QueryError::InvalidArgument(_)));

            assert_eq!(
                error.to_string(),
                "invalid argument error: offset out of bounds"
            );
        }
    }
}
