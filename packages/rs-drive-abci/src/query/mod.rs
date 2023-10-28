mod data_contract_based_queries;
mod document_query;
mod identity_based_queries;
mod proofs;
mod response_metadata;
mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::error::execution::ExecutionError;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        match platform_version.drive_abci.query.base_query_structure {
            0 => self.query_v0(query_path, query_data, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "Platform::query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! extract_variant_or_panic {
        ($expression:expr, $pattern:pat, $binding:ident) => {
            match $expression {
                $pattern => $binding,
                // _ => panic!(
                //     "Expected pattern {} but got another variant",
                //     stringify!($pattern)
                // ),
            }
        };
    }

    use crate::config::PlatformConfig;
    use crate::error::query::QueryError;
    use crate::platform_types::platform::Platform;
    use crate::query::QueryValidationResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::prelude::DataContract;
    use drive::drive::batch::DataContractOperationType;
    use drive::drive::batch::DriveOperation::DataContractOperation;
    use platform_version::version::PlatformVersion;
    use std::borrow::Cow;

    fn setup_platform<'a>() -> (TempPlatform<MockCoreRPCLike>, &'a PlatformVersion) {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                ..Default::default()
            })
            .build_with_mock_rpc();
        let platform = platform.set_initial_state_structure();

        let platform_version = platform
            .platform
            .state
            .read()
            .unwrap()
            .current_platform_version()
            .unwrap();

        (platform, platform_version)
    }

    fn store_data_contract(
        platform: &Platform<MockCoreRPCLike>,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) {
        let operation = DataContractOperation(DataContractOperationType::ApplyContract {
            contract: Cow::Owned(data_contract.to_owned()),
            storage_flags: None,
        });

        let block_info = BlockInfo {
            time_ms: 0,
            height: 0,
            core_height: 0,
            epoch: Epoch::default(),
        };

        platform
            .drive
            .apply_drive_operations(vec![operation], true, &block_info, None, platform_version)
            .expect("expected to apply drive operations");
    }

    fn assert_invalid_identifier(validation_result: QueryValidationResult<Vec<u8>>) {
        let validation_error = validation_result.first_error().unwrap();

        assert!(matches!(
            validation_error,
            QueryError::InvalidArgument(msg) if msg.contains("id must be a valid identifier (32 bytes long)")
        ));
    }

    #[test]
    fn test_invalid_path() {
        let (platform, platform_version) = setup_platform();

        let data = vec![0; 32];
        let path = "/invalid_path";
        let result = platform.query(path, &data, platform_version);
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        assert!(matches!(
            validation_result.first_error().unwrap(),
            QueryError::InvalidArgument(msg) if msg == &format!("query path '{}' is not supported", path).to_string()
        ));
    }

    #[test]
    fn test_invalid_query_data() {
        let (platform, version) = setup_platform();

        let paths = vec![
            "/identity",
            "/identities",
            "/identity/balance",
            "/identity/balanceAndRevision",
            "/identity/keys",
            "/dataContract",
            "/dataContracts",
            "/dataContractHistory",
            "/documents",
            "/dataContract/documents",
            "/identity/by-public-key-hash",
            "/identities/by-public-key-hash",
            "/proofs",
        ];

        paths.iter().for_each(|path| {
                let result = platform.query(path, &[0; 8], version);
                assert!(result.is_ok());

                let validation_result = result.unwrap();
                let validation_error = validation_result.first_error().unwrap();

                assert!(
                    matches!(validation_error, QueryError::InvalidArgument(msg) if msg.contains("invalid query proto message"))
                );
            })
    }

    mod identity {
        use crate::error::query::QueryError;
        use crate::query::tests::assert_invalid_identifier;
        use bs58::encode;
        use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
        use dapi_grpc::platform::v0::{
            get_identity_request, get_identity_response, GetIdentityRequest, GetIdentityResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/identity";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityRequest {
                version: Some(get_identity_request::Version::V0(GetIdentityRequestV0 {
                    id: vec![0; 8],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_identity_not_found() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetIdentityRequest {
                version: Some(get_identity_request::Version::V0(GetIdentityRequestV0 {
                    id: id.clone(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            let error_message = format!("identity {} not found", encode(id).into_string());

            assert!(matches!(
                validation_error,
                QueryError::NotFound(msg) if msg.contains(&error_message)
            ));
        }

        #[test]
        fn test_identity_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetIdentityRequest {
                version: Some(get_identity_request::Version::V0(GetIdentityRequestV0 {
                    id: id.clone(),
                    prove: true,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let response =
                GetIdentityResponse::decode(validation_result.data.unwrap().as_slice()).unwrap();

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identity_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_identity_response::get_identity_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identities {
        use dapi_grpc::platform::v0::get_identities_request::GetIdentitiesRequestV0;
        use dapi_grpc::platform::v0::{
            get_identities_request, get_identities_response, GetIdentitiesRequest,
            GetIdentitiesResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/identities";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentitiesRequest {
                version: Some(get_identities_request::Version::V0(
                    GetIdentitiesRequestV0 {
                        ids: vec![vec![0; 8]],
                        prove: false,
                    },
                )),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_identities_not_found() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentitiesRequest {
                version: Some(get_identities_request::Version::V0(
                    GetIdentitiesRequestV0 {
                        ids: vec![id.clone()],
                        prove: false,
                    },
                )),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();
            let data = validation_result.data.unwrap();
            let response = GetIdentitiesResponse::decode(data.as_slice()).unwrap();

            let get_identities_response::get_identities_response_v0::Result::Identities(identities) =
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identities_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap()
            else {
                panic!("invalid response")
            };

            assert!(identities.identity_entries[0].value.is_none());
        }

        #[test]
        fn test_identities_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetIdentitiesRequest {
                version: Some(get_identities_request::Version::V0(
                    GetIdentitiesRequestV0 {
                        ids: vec![id.clone()],
                        prove: true,
                    },
                )),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let response =
                GetIdentitiesResponse::decode(validation_result.data.unwrap().as_slice()).unwrap();

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identities_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_identities_response::get_identities_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identity_balance {
        use crate::error::query::QueryError;
        use bs58::encode;
        use dapi_grpc::platform::v0::get_identity_balance_request::{
            GetIdentityBalanceRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identity_balance_response, GetIdentityBalanceRequest, GetIdentityBalanceResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/identity/balance";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityBalanceRequest {
                version: Some(Version::V0(GetIdentityBalanceRequestV0 {
                    id: vec![0; 8],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_identity_not_found() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentityBalanceRequest {
                version: Some(Version::V0(GetIdentityBalanceRequestV0 {
                    id: id.clone(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let validation_error = validation_result.first_error().unwrap();

            assert_eq!(
                validation_error.to_string(),
                "not found error: No Identity found".to_string()
            );

            assert!(matches!(validation_error, QueryError::NotFound(_)));
        }

        #[test]
        fn test_identity_balance_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentityBalanceRequest {
                version: Some(Version::V0(GetIdentityBalanceRequestV0 {
                    id: id.clone(),
                    prove: true,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();
            let response =
                GetIdentityBalanceResponse::decode(validation_result.data.unwrap().as_slice())
                    .unwrap();

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identity_balance_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_identity_balance_response::get_identity_balance_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identity_balance_and_revision {
        use crate::error::query::QueryError;
        use bs58::encode;
        use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::{
            GetIdentityBalanceAndRevisionRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identity_balance_and_revision_response, GetIdentityBalanceAndRevisionRequest,
            GetIdentityBalanceAndRevisionResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/identity/balanceAndRevision";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityBalanceAndRevisionRequest {
                version: Some(Version::V0(GetIdentityBalanceAndRevisionRequestV0 {
                    id: vec![0; 8],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_identity_not_found_when_querying_balance_and_revision() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentityBalanceAndRevisionRequest {
                version: Some(Version::V0(GetIdentityBalanceAndRevisionRequestV0 {
                    id: id.clone(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let validation_error = validation_result.first_error().unwrap();

            assert_eq!(
                validation_error.to_string(),
                "not found error: No Identity balance found".to_string()
            );

            assert!(matches!(validation_error, QueryError::NotFound(_)));
        }

        #[test]
        fn test_identity_balance_and_revision_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentityBalanceAndRevisionRequest {
                version: Some(Version::V0(GetIdentityBalanceAndRevisionRequestV0 {
                    id: id.clone(),
                    prove: true,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let response = GetIdentityBalanceAndRevisionResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            assert!(matches!(
                extract_variant_or_panic!(response.version.expect("expected a versioned response"), get_identity_balance_and_revision_response::Version::V0(inner), inner).result.unwrap(),
                get_identity_balance_and_revision_response::get_identity_balance_and_revision_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identity_keys {
        use crate::error::query::QueryError;
        use dapi_grpc::platform::v0::get_identity_keys_request::{
            GetIdentityKeysRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identity_keys_response, key_request_type::Request, AllKeys, GetIdentityKeysRequest,
            GetIdentityKeysResponse, KeyRequestType, SearchKey, SecurityLevelMap,
        };
        use drive::error::query::QuerySyntaxError;
        use prost::Message;

        const QUERY_PATH: &str = "/identity/keys";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 8],
                    request_type: None,
                    limit: None,
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_invalid_limit_u16_overflow() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: None,
                    limit: Some(u32::MAX),
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::InvalidParameter(msg)) if msg == "limit out of bounds"
            ));
        }

        #[test]
        fn test_invalid_limit_max() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: None,
                    limit: Some((platform.config.drive.max_query_limit + 1) as u32),
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            let error_message = format!(
                "limit greater than max limit {}",
                platform.config.drive.max_query_limit
            );

            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::InvalidLimit(msg)) if msg == &error_message
            ));
        }

        #[test]
        fn test_invalid_offset_u16_overflow() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: None,
                    limit: None,
                    offset: Some(u32::MAX),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::InvalidParameter(msg)) if msg == "offset out of bounds"
            ));
        }

        #[test]
        fn test_missing_request_type() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: None,
                    limit: None,
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::InvalidParameter(msg)) if msg == "key request must be defined"
            ));
        }

        #[test]
        fn test_missing_request() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: Some(KeyRequestType { request: None }),
                    limit: None,
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::InvalidParameter(msg)) if msg == "key request must be defined"
            ));
        }

        #[test]
        fn test_invalid_key_request_type() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: Some(KeyRequestType {
                        request: Some(Request::SearchKey(SearchKey {
                            purpose_map: [(
                                0,
                                SecurityLevelMap {
                                    security_level_map: [(u32::MAX, 0)].into_iter().collect(),
                                },
                            )]
                            .into_iter()
                            .collect(),
                        })),
                    }),
                    limit: None,
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg)) if msg == "security level out of bounds"
            ));
        }

        #[test]
        fn test_absent_keys() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: Some(KeyRequestType {
                        request: Some(Request::AllKeys(AllKeys {})),
                    }),
                    limit: None,
                    offset: None,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();
            let data = validation_result.data.unwrap();

            let response = GetIdentityKeysResponse::decode(data.as_slice()).unwrap();
            let get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(keys) =
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identity_keys_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap()
            else {
                panic!("invalid response")
            };

            assert_eq!(keys.keys_bytes.len(), 0);
        }

        #[test]
        fn test_absent_keys_proof() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityKeysRequest {
                version: Some(Version::V0(GetIdentityKeysRequestV0 {
                    identity_id: vec![0; 32],
                    request_type: Some(KeyRequestType {
                        request: Some(Request::AllKeys(AllKeys {})),
                    }),
                    limit: None,
                    offset: None,
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let response =
                GetIdentityKeysResponse::decode(validation_result.data.unwrap().as_slice())
                    .unwrap();

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identity_keys_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_identity_keys_response::get_identity_keys_response_v0::Result::Proof(_)
            ));
        }
    }

    mod data_contract {
        use crate::error::query::QueryError;
        use bs58::encode;
        use dapi_grpc::platform::v0::get_data_contract_request::{
            GetDataContractRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_data_contract_response, GetDataContractRequest, GetDataContractResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/dataContract";

        #[test]
        fn test_invalid_data_contract_id() {
            let (platform, version) = super::setup_platform();

            let request = GetDataContractRequest {
                version: Some(Version::V0(GetDataContractRequestV0 {
                    id: vec![0; 8],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_data_contract_not_found() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetDataContractRequest {
                version: Some(Version::V0(GetDataContractRequestV0 {
                    id: id.clone(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            let error_message = format!("data contract {} not found", encode(id).into_string());

            assert!(matches!(
                validation_error,
                QueryError::NotFound(msg) if msg.contains(&error_message)
            ));
        }

        #[test]
        fn test_data_contract_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetDataContractRequest {
                version: Some(Version::V0(GetDataContractRequestV0 {
                    id: id.clone(),
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let response =
                GetDataContractResponse::decode(validation_result.data.unwrap().as_slice())
                    .unwrap();

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_data_contract_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_data_contract_response::get_data_contract_response_v0::Result::Proof(_)
            ));
        }
    }

    mod data_contracts {
        use dapi_grpc::platform::v0::get_data_contracts_request::{
            GetDataContractsRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_data_contracts_response, GetDataContractsRequest, GetDataContractsResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/dataContracts";

        #[test]
        fn test_invalid_data_contract_id() {
            let (platform, version) = super::setup_platform();

            let request = GetDataContractsRequest {
                version: Some(Version::V0(GetDataContractsRequestV0 {
                    ids: vec![vec![0; 8]],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_data_contracts_not_found() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetDataContractsRequest {
                version: Some(Version::V0(GetDataContractsRequestV0 {
                    ids: vec![id.clone()],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();
            let data = validation_result.data.unwrap();
            let response = GetDataContractsResponse::decode(data.as_slice()).unwrap();

            let get_data_contracts_response::get_data_contracts_response_v0::Result::DataContracts(
                contracts,
            ) = extract_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_data_contracts_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a response")
            else {
                panic!("invalid response")
            };

            assert!(contracts.data_contract_entries[0].data_contract.is_none());
        }

        #[test]
        fn test_data_contracts_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];
            let request = GetDataContractsRequest {
                version: Some(Version::V0(GetDataContractsRequestV0 {
                    ids: vec![id.clone()],
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let response =
                GetDataContractsResponse::decode(validation_result.data.unwrap().as_slice())
                    .unwrap();

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_data_contracts_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_data_contracts_response::get_data_contracts_response_v0::Result::Proof(_)
            ));
        }
    }

    mod data_contract_history {
        use crate::error::query::QueryError;
        use bs58::encode;
        use dapi_grpc::platform::v0::get_data_contract_history_request::{
            GetDataContractHistoryRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_data_contract_history_response, GetDataContractHistoryRequest,
            GetDataContractHistoryResponse,
        };
        use prost::Message;

        const QUERY_PATH: &str = "/dataContractHistory";

        #[test]
        fn test_invalid_data_contract_id() {
            let (platform, version) = super::setup_platform();

            let request = GetDataContractHistoryRequest {
                version: Some(Version::V0(GetDataContractHistoryRequestV0 {
                    id: vec![0; 8],
                    limit: None,
                    offset: None,
                    start_at_ms: 0,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_invalid_limit_overflow() {
            let (platform, version) = super::setup_platform();

            let request = GetDataContractHistoryRequest {
                version: Some(Version::V0(GetDataContractHistoryRequestV0 {
                    id: vec![0; 32],
                    limit: Some(u32::MAX),
                    offset: None,
                    start_at_ms: 0,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(validation_error, QueryError::InvalidArgument(_)));
            assert_eq!(
                validation_error.to_string().as_str(),
                "invalid argument error: limit out of bounds"
            )
        }

        #[test]
        fn test_invalid_offset_overflow() {
            let (platform, version) = super::setup_platform();

            let request = GetDataContractHistoryRequest {
                version: Some(Version::V0(GetDataContractHistoryRequestV0 {
                    id: vec![0; 32],
                    limit: None,
                    offset: Some(u32::MAX),
                    start_at_ms: 0,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let validation_error = validation_result.first_error().unwrap();

            assert!(matches!(validation_error, QueryError::InvalidArgument(_)));
            assert_eq!(
                validation_error.to_string().as_str(),
                "invalid argument error: offset out of bounds"
            );
        }

        #[test]
        fn test_data_contract_not_found() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetDataContractHistoryRequest {
                version: Some(Version::V0(GetDataContractHistoryRequestV0 {
                    id: vec![0; 32],
                    limit: None,
                    offset: None,
                    start_at_ms: 0,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            let validation_error = validation_result.first_error().unwrap();

            let error_message = format!(
                "data contract {} history not found",
                encode(id).into_string()
            );

            assert!(matches!(
                validation_error,
                QueryError::NotFound(msg) if msg.contains(&error_message)
            ));
        }

        #[test]
        fn test_data_contract_history_absence_proof() {
            let (platform, version) = super::setup_platform();

            let request = GetDataContractHistoryRequest {
                version: Some(Version::V0(GetDataContractHistoryRequestV0 {
                    id: vec![0; 32],
                    limit: None,
                    offset: None,
                    start_at_ms: 0,
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let response =
                GetDataContractHistoryResponse::decode(validation_result.data.unwrap().as_slice())
                    .unwrap();

            assert!(matches!(
                extract_variant_or_panic!(response.version.expect("expected a versioned response"), get_data_contract_history_response::Version::V0(inner), inner).result.unwrap(),
                get_data_contract_history_response::get_data_contract_history_response_v0::Result::Proof(_)
            ));
        }
    }

    mod documents {
        use crate::error::query::QueryError;
        use crate::query::tests::store_data_contract;
        use dapi_grpc::platform::v0::{
            get_documents_response, GetDocumentsRequest, GetDocumentsResponse,
        };

        use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
        use dapi_grpc::platform::v0::get_documents_request::{GetDocumentsRequestV0, Version};
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::platform_value::string_encoding::Encoding;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use drive::error::query::QuerySyntaxError;
        use prost::Message;

        const QUERY_PATH: &str = "/documents";

        #[test]
        fn test_invalid_document_id() {
            let (platform, version) = super::setup_platform();

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: vec![0; 8],
                    document_type: "niceDocument".to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_data_contract_not_found_in_documents_request() {
            let (platform, version) = super::setup_platform();

            let data_contract_id = vec![0; 32];

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.clone(),
                    document_type: "niceDocument".to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");
            let validation_error = validation_result.first_error().expect("expected an error");
            assert_eq!(validation_error.to_string().as_str(), "query syntax error: contract not found error: contract not found when querying from value with contract info");
            assert!(matches!(
                validation_error,
                QueryError::Query(QuerySyntaxError::DataContractNotFound(_))
            ));
        }

        #[test]
        fn test_absent_document_type() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "fakeDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();

            let message = format!(
                "document type {} not found for contract {}",
                document_type,
                data_contract_id.to_string(Encoding::Base58)
            );
            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::InvalidArgument(msg) if msg.contains(message.as_str())
            ))
        }

        #[test]
        fn test_invalid_where_clause() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![0x9F], // Incomplete CBOR array
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::Query(QuerySyntaxError::DeserializationError(msg)) if msg == &"unable to decode 'where' query from cbor".to_string()
            ))
        }

        #[test]
        fn test_invalid_order_by_clause() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![0x9F], // Incomplete CBOR array
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::Query(QuerySyntaxError::DeserializationError(msg)) if msg == &"unable to decode 'order_by' query from cbor".to_string()
            ))
        }

        #[test]
        fn test_invalid_start_at_clause() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: Some(Start::StartAt(vec![0; 8])),
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg)) if msg == &"start at should be a 32 byte identifier".to_string()
            ))
        }

        #[test]
        fn test_invalid_start_after_clause() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: Some(Start::StartAfter(vec![0; 8])),
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg)) if msg == &"start after should be a 32 byte identifier".to_string()
            ))
        }

        #[test]
        fn test_invalid_limit() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";
            let limit = u32::MAX;

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit,
                    order_by: vec![],
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::Query(QuerySyntaxError::InvalidLimit(msg)) if msg == &format!("limit {} out of bounds", limit).to_string()
            ))
        }

        #[test]
        fn test_documents_not_found() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: false,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            assert!(validation_result.is_valid());
            let response = GetDocumentsResponse::decode(
                validation_result.data.expect("data must exist").as_slice(),
            )
            .expect("response decoded");

            let Some(result) = extract_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_documents_response::Version::V0(inner),
                inner
            )
            .result
            else {
                panic!("invalid response")
            };

            let get_documents_response::get_documents_response_v0::Result::Documents(documents) =
                result
            else {
                panic!("invalid response")
            };

            assert_eq!(documents.documents.len(), 0);
        }

        #[test]
        fn test_documents_absence_proof() {
            let (platform, version) = super::setup_platform();

            let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
            store_data_contract(&platform, created_data_contract.data_contract(), version);

            let data_contract_id = created_data_contract.data_contract().id();
            let document_type = "niceDocument";

            let request = GetDocumentsRequest {
                version: Some(Version::V0(GetDocumentsRequestV0 {
                    data_contract_id: data_contract_id.to_vec(),
                    document_type: document_type.to_string(),
                    r#where: vec![],
                    limit: 0,
                    order_by: vec![],
                    prove: true,
                    start: None,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();
            assert!(validation_result.is_valid());
            let response = GetDocumentsResponse::decode(
                validation_result.data.expect("data must exist").as_slice(),
            )
            .expect("response decoded");

            assert!(matches!(
                extract_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_documents_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_documents_response::get_documents_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identity_by_public_key_hash {
        use crate::query::QueryError;
        use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::{
            GetIdentityByPublicKeyHashRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identity_by_public_key_hash_response, GetIdentityByPublicKeyHashRequest,
            GetIdentityByPublicKeyHashResponse,
        };
        use prost::Message;

        const PATH: &str = "/identity/by-public-key-hash";

        #[test]
        fn test_invalid_public_key_hash() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityByPublicKeyHashRequest {
                version: Some(Version::V0(GetIdentityByPublicKeyHashRequestV0 {
                    public_key_hash: vec![0; 8],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::InvalidArgument(msg) if msg == &"public key hash must be 20 bytes long".to_string()
            ));
        }

        #[test]
        fn test_identity_not_found() {
            let (platform, version) = super::setup_platform();

            let public_key_hash = vec![0; 20];
            let request = GetIdentityByPublicKeyHashRequest {
                version: Some(Version::V0(GetIdentityByPublicKeyHashRequestV0 {
                    public_key_hash: public_key_hash.clone(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::NotFound(msg) if msg == &format!("identity for public key hash {} not found", hex::encode(public_key_hash.as_slice())).to_string()
            ))
        }

        #[test]
        fn test_identity_absence_proof() {
            let (platform, version) = super::setup_platform();

            let public_key_hash = vec![0; 20];
            let request = GetIdentityByPublicKeyHashRequest {
                version: Some(Version::V0(GetIdentityByPublicKeyHashRequestV0 {
                    public_key_hash: public_key_hash.clone(),
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");

            let response = GetIdentityByPublicKeyHashResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .expect("response decoded");

            assert!(matches!(
                extract_variant_or_panic!(response.version.expect("expected a versioned response"), get_identity_by_public_key_hash_response::Version::V0(inner), inner).result.unwrap(),
                get_identity_by_public_key_hash_response::get_identity_by_public_key_hash_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identities_by_public_key_hash {
        use crate::query::QueryError;
        use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_request::{
            GetIdentitiesByPublicKeyHashesRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identities_by_public_key_hashes_response, GetIdentitiesByPublicKeyHashesRequest,
            GetIdentitiesByPublicKeyHashesResponse,
        };
        use prost::Message;

        const PATH: &str = "/identities/by-public-key-hash";

        #[test]
        fn test_invalid_public_key_hash() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentitiesByPublicKeyHashesRequest {
                version: Some(Version::V0(GetIdentitiesByPublicKeyHashesRequestV0 {
                    public_key_hashes: vec![vec![0; 8]],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::InvalidArgument(msg) if msg == &"public key hash must be 20 bytes long".to_string()
            ));
        }

        #[test]
        fn test_identities_not_found() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentitiesByPublicKeyHashesRequest {
                version: Some(Version::V0(GetIdentitiesByPublicKeyHashesRequestV0 {
                    public_key_hashes: vec![vec![0; 20]],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetIdentitiesByPublicKeyHashesResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .expect("response decoded");

            let get_identities_by_public_key_hashes_response::get_identities_by_public_key_hashes_response_v0::Result::Identities(identities) =
                extract_variant_or_panic!(response.version.expect("expected a versioned response"), get_identities_by_public_key_hashes_response::Version::V0(inner), inner).result.expect("expected a versioned result")
            else {
                panic!("invalid response")
            };

            assert!(identities.identity_entries.first().unwrap().value.is_none());
        }

        #[test]
        fn test_identities_absence_proof() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentitiesByPublicKeyHashesRequest {
                version: Some(Version::V0(GetIdentitiesByPublicKeyHashesRequestV0 {
                    public_key_hashes: vec![vec![0; 20]],
                    prove: true,
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();
            let response = GetIdentitiesByPublicKeyHashesResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .expect("response decoded");

            assert!(matches!(
                extract_variant_or_panic!(response.version.expect("expected a versioned response"), get_identities_by_public_key_hashes_response::Version::V0(inner), inner).result.unwrap(),
                get_identities_by_public_key_hashes_response::get_identities_by_public_key_hashes_response_v0::Result::Proof(_)
            ));
        }
    }

    mod proofs {
        use crate::query::QueryError;
        use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::{
            ContractRequest, DocumentRequest, IdentityRequest,
        };
        use dapi_grpc::platform::v0::get_proofs_request::{GetProofsRequestV0, Version};
        use dapi_grpc::platform::v0::{get_proofs_response, GetProofsRequest, GetProofsResponse};
        use prost::Message;

        const PATH: &str = "/proofs";

        #[test]
        fn test_invalid_identity_ids() {
            let (platform, version) = super::setup_platform();

            let request = GetProofsRequest {
                version: Some(Version::V0(GetProofsRequestV0 {
                    identities: vec![IdentityRequest {
                        identity_id: vec![0; 8],
                        request_type: 0,
                    }],
                    contracts: vec![],
                    documents: vec![],
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_invalid_identity_prove_request_type() {
            let (platform, version) = super::setup_platform();

            let request_type = 10;

            let request = GetProofsRequest {
                version: Some(Version::V0(GetProofsRequestV0 {
                    identities: vec![IdentityRequest {
                        identity_id: vec![0; 32],
                        request_type,
                    }],
                    contracts: vec![],
                    documents: vec![],
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());
            let validation_result = result.unwrap();

            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::InvalidArgument(msg) if msg == &format!(
                    "invalid prove request type '{}'",
                    request_type
                ).to_string()
            ))
        }

        #[test]
        fn test_invalid_contract_ids() {
            let (platform, version) = super::setup_platform();

            let request = GetProofsRequest {
                version: Some(Version::V0(GetProofsRequestV0 {
                    identities: vec![],
                    contracts: vec![ContractRequest {
                        contract_id: vec![0; 8],
                    }],
                    documents: vec![],
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_invalid_contract_id_for_documents_proof() {
            let (platform, version) = super::setup_platform();

            let request = GetProofsRequest {
                version: Some(Version::V0(GetProofsRequestV0 {
                    identities: vec![],
                    contracts: vec![],
                    documents: vec![DocumentRequest {
                        contract_id: vec![0; 8],
                        document_type: "niceDocument".to_string(),
                        document_type_keeps_history: false,
                        document_id: vec![0; 32],
                    }],
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_invalid_document_id() {
            let (platform, version) = super::setup_platform();

            let request = GetProofsRequest {
                version: Some(Version::V0(GetProofsRequestV0 {
                    identities: vec![],
                    contracts: vec![],
                    documents: vec![DocumentRequest {
                        contract_id: vec![0; 32],
                        document_type: "niceDocument".to_string(),
                        document_type_keeps_history: false,
                        document_id: vec![0; 8],
                    }],
                })),
            }
            .encode_to_vec();

            let result = platform.query(PATH, &request, version);
            assert!(result.is_ok());
            super::assert_invalid_identifier(result.unwrap());
        }

        #[test]
        fn test_proof_of_absence() {
            let (platform, version) = super::setup_platform();

            let request = GetProofsRequest {
                version: Some(Version::V0(GetProofsRequestV0 {
                    identities: vec![],
                    contracts: vec![],
                    documents: vec![DocumentRequest {
                        contract_id: vec![0; 32],
                        document_type: "niceDocument".to_string(),
                        document_type_keeps_history: false,
                        document_id: vec![0; 32],
                    }],
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response =
                GetProofsResponse::decode(validation_result.data.unwrap().as_slice()).unwrap();

            let proof = extract_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_proofs_response::Version::V0(inner),
                inner
            )
            .proof;

            assert!(proof.is_some())
        }
    }
}
