mod data_contract_based_queries;
mod document_query;
mod identity_based_queries;
mod proofs;
mod response_metadata;
mod system;
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
    macro_rules! extract_single_variant_or_panic {
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

    #[macro_export]
    macro_rules! extract_variant_or_panic {
        ($expression:expr, $pattern:pat, $binding:ident) => {
            match $expression {
                $pattern => $binding,
                _ => panic!(
                    "Expected pattern {} but got another variant",
                    stringify!($pattern)
                ),
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
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(
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
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(
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
                extract_single_variant_or_panic!(
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

        use dapi_grpc::platform::v0::get_identity_balance_request::{
            GetIdentityBalanceRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identity_balance_response, GetIdentityBalanceRequest, GetIdentityBalanceResponse,
        };
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(
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

    mod identity_nonce {
        use crate::error::query::QueryError;

        use dapi_grpc::platform::v0::get_identity_nonce_request::{
            GetIdentityNonceRequestV0, Version,
        };
        use dapi_grpc::platform::v0::get_identity_nonce_response::get_identity_nonce_response_v0;
        use dapi_grpc::platform::v0::{
            get_identity_nonce_response, GetIdentityNonceRequest, GetIdentityNonceResponse,
        };
        use dapi_grpc::Message;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::document_type::random_document::CreateRandomDocument;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::identity_nonce::{
            IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES,
        };
        use drive::common::identities::create_test_identity_with_rng;
        use rand::prelude::StdRng;
        use rand::{Rng, SeedableRng};

        const QUERY_PATH: &str = "/identity/nonce";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityNonceRequest {
                version: Some(Version::V0(GetIdentityNonceRequestV0 {
                    identity_id: vec![0; 8],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_error = result.as_ref().unwrap().first_error().unwrap();

            assert!(matches!(
            validation_error,
            QueryError::InvalidArgument(msg) if msg.contains("identity id must be 32 bytes long")));
        }

        #[test]
        fn test_identity_not_found_when_querying_identity_nonce() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityNonceRequest {
                version: Some(Version::V0(GetIdentityNonceRequestV0 {
                    identity_id: vec![0; 32],
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityNonceResponse { version } =
                GetIdentityNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_nonce_response_v0::Result::IdentityNonce(nonce) => nonce,
                get_identity_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 0);
        }

        #[test]
        fn test_identity_is_found_when_querying_identity_nonce() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(10);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            let request = GetIdentityNonceRequest {
                version: Some(Version::V0(GetIdentityNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityNonceResponse { version } =
                GetIdentityNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_nonce_response_v0::Result::IdentityNonce(nonce) => nonce,
                get_identity_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 0);
        }

        #[test]
        fn test_identity_is_found_when_querying_identity_nonce_after_update() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(10);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            let cache = platform.drive.cache.read().unwrap();

            let dashpay = &cache.system_data_contracts.dashpay;

            platform
                .drive
                .merge_identity_nonce(
                    identity.id().to_buffer(),
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("expected to set nonce");

            platform
                .drive
                .merge_identity_nonce(
                    identity.id().to_buffer(),
                    3,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("expected to set nonce");

            let request = GetIdentityNonceRequest {
                version: Some(Version::V0(GetIdentityNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityNonceResponse { version } =
                GetIdentityNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_nonce_response_v0::Result::IdentityNonce(nonce) => nonce,
                get_identity_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce & IDENTITY_NONCE_VALUE_FILTER, 3);
            assert_eq!(nonce >> IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES, 1);
            // the previous last one was not there
        }

        #[test]
        fn test_identity_is_found_when_querying_identity_nonce_after_update_for_past() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(10);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            platform
                .drive
                .merge_identity_nonce(
                    identity.id().to_buffer(),
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("expected to set nonce");

            platform
                .drive
                .merge_identity_nonce(
                    identity.id().to_buffer(),
                    3,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("expected to set nonce");

            // we already added 3, and now are adding 2
            platform
                .drive
                .merge_identity_nonce(
                    identity.id().to_buffer(),
                    2,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("expected to set nonce");

            let request = GetIdentityNonceRequest {
                version: Some(Version::V0(GetIdentityNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityNonceResponse { version } =
                GetIdentityNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_nonce_response_v0::Result::IdentityNonce(nonce) => nonce,
                get_identity_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 3);
        }

        #[test]
        fn test_identity_contract_nonce_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentityNonceRequest {
                version: Some(Version::V0(GetIdentityNonceRequestV0 {
                    identity_id: vec![0; 32],
                    prove: true,
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();
            let response =
                GetIdentityNonceResponse::decode(validation_result.data.unwrap().as_slice())
                    .unwrap();

            assert!(matches!(
                extract_single_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identity_nonce_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_identity_nonce_response::get_identity_nonce_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identity_contract_nonce {
        use crate::error::query::QueryError;
        use std::collections::BTreeMap;

        use dapi_grpc::platform::v0::get_identity_contract_nonce_request::{
            GetIdentityContractNonceRequestV0, Version,
        };
        use dapi_grpc::platform::v0::get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0;
        use dapi_grpc::platform::v0::{
            get_identity_contract_nonce_response, GetIdentityContractNonceRequest,
            GetIdentityContractNonceResponse,
        };
        use dapi_grpc::Message;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::random_document::CreateRandomDocument;
        use dpp::identifier::Identifier;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::identity_nonce::{
            IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES,
        };
        use dpp::identity::{Identity, IdentityV0};
        use dpp::prelude::IdentityNonce;
        use drive::common::identities::{
            create_test_identities_with_rng, create_test_identity_with_rng,
        };
        use drive::drive::object_size_info::DocumentInfo::DocumentRefInfo;
        use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
        use rand::prelude::StdRng;
        use rand::{Rng, SeedableRng};

        const QUERY_PATH: &str = "/identity/contractNonce";

        #[test]
        fn test_invalid_identity_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: vec![0; 8],
                    prove: false,
                    contract_id: vec![1; 32],
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_error = result.as_ref().unwrap().first_error().unwrap();

            assert!(matches!(
            validation_error,
            QueryError::InvalidArgument(msg) if msg.contains("identity id must be 32 bytes long")));
        }

        #[test]
        fn test_invalid_contract_id() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: vec![0; 32],
                    prove: false,
                    contract_id: vec![1; 8],
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            assert!(result.is_ok());
            let validation_error = result.as_ref().unwrap().first_error().unwrap();

            assert!(matches!(
            validation_error,
            QueryError::InvalidArgument(msg) if msg.contains("contract id must be 32 bytes long")));
        }

        #[test]
        fn test_identity_not_found_when_querying_identity_nonce() {
            let (platform, version) = super::setup_platform();

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: vec![0; 32],
                    prove: false,
                    contract_id: vec![0; 32],
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityContractNonceResponse { version } =
                GetIdentityContractNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_contract_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_contract_nonce_response_v0::Result::IdentityContractNonce(nonce) => {
                    nonce
                }
                get_identity_contract_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 0);
        }

        #[test]
        fn test_contract_info_not_found_when_querying_identity_nonce_with_known_identity() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(45);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                    contract_id: vec![0; 32],
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityContractNonceResponse { version } =
                GetIdentityContractNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_contract_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_contract_nonce_response_v0::Result::IdentityContractNonce(nonce) => {
                    nonce
                }
                get_identity_contract_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 0);
        }

        #[test]
        fn test_identity_is_found_when_querying_identity_nonce() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(10);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            let cache = platform.drive.cache.read().unwrap();

            let dashpay = &cache.system_data_contracts.dashpay;

            platform
                .drive
                .merge_revision_nonce_for_identity_contract_pair(
                    identity.id().to_buffer(),
                    dashpay.id().to_buffer(),
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    version,
                )
                .expect("expected to set nonce");

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                    contract_id: dashpay.id().to_vec(),
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityContractNonceResponse { version } =
                GetIdentityContractNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_contract_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_contract_nonce_response_v0::Result::IdentityContractNonce(nonce) => {
                    nonce
                }
                get_identity_contract_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 1);
        }

        #[test]
        fn test_identity_is_found_when_querying_identity_nonce_after_update() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(10);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            let cache = platform.drive.cache.read().unwrap();

            let dashpay = &cache.system_data_contracts.dashpay;

            platform
                .drive
                .merge_revision_nonce_for_identity_contract_pair(
                    identity.id().to_buffer(),
                    dashpay.id().to_buffer(),
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    version,
                )
                .expect("expected to set nonce");

            platform
                .drive
                .merge_revision_nonce_for_identity_contract_pair(
                    identity.id().to_buffer(),
                    dashpay.id().to_buffer(),
                    3,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    version,
                )
                .expect("expected to set nonce");

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                    contract_id: dashpay.id().to_vec(),
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityContractNonceResponse { version } =
                GetIdentityContractNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_contract_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_contract_nonce_response_v0::Result::IdentityContractNonce(nonce) => {
                    nonce
                }
                get_identity_contract_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce & IDENTITY_NONCE_VALUE_FILTER, 3);
            assert_eq!(nonce >> IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES, 1);
            // the previous last one was not there
        }

        #[test]
        fn test_identity_is_found_when_querying_identity_nonce_after_update_for_past() {
            let (platform, version) = super::setup_platform();
            let mut rng = StdRng::seed_from_u64(10);
            let id = rng.gen::<[u8; 32]>();
            let identity =
                create_test_identity_with_rng(&platform.drive, id, &mut rng, None, version)
                    .expect("expected to create a test identity");

            let cache = platform.drive.cache.read().unwrap();

            let dashpay = &cache.system_data_contracts.dashpay;

            platform
                .drive
                .merge_revision_nonce_for_identity_contract_pair(
                    identity.id().to_buffer(),
                    dashpay.id().to_buffer(),
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    version,
                )
                .expect("expected to set nonce");

            platform
                .drive
                .merge_revision_nonce_for_identity_contract_pair(
                    identity.id().to_buffer(),
                    dashpay.id().to_buffer(),
                    3,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    version,
                )
                .expect("expected to set nonce");

            // we already added 3, and now are adding 2
            platform
                .drive
                .merge_revision_nonce_for_identity_contract_pair(
                    identity.id().to_buffer(),
                    dashpay.id().to_buffer(),
                    2,
                    &BlockInfo::default(),
                    true,
                    None,
                    &mut vec![],
                    version,
                )
                .expect("expected to set nonce");

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: id.to_vec(),
                    prove: false,
                    contract_id: dashpay.id().to_vec(),
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(QUERY_PATH, &request, version)
                .expect("expected query to succeed");

            assert!(validation_result.first_error().is_none());

            let response_data = validation_result.into_data().expect("expected data");

            let GetIdentityContractNonceResponse { version } =
                GetIdentityContractNonceResponse::decode(response_data.as_slice())
                    .expect("expected to decode the response");

            let v0 = match version.unwrap() {
                get_identity_contract_nonce_response::Version::V0(v0) => v0,
            };

            let nonce = match v0.result.unwrap() {
                get_identity_contract_nonce_response_v0::Result::IdentityContractNonce(nonce) => {
                    nonce
                }
                get_identity_contract_nonce_response_v0::Result::Proof(_) => {
                    panic!("expected non proved")
                }
            };

            assert_eq!(nonce, 3);
        }

        #[test]
        fn test_identity_contract_nonce_absence_proof() {
            let (platform, version) = super::setup_platform();

            let id = vec![0; 32];

            let request = GetIdentityContractNonceRequest {
                version: Some(Version::V0(GetIdentityContractNonceRequestV0 {
                    identity_id: vec![0; 32],
                    prove: true,
                    contract_id: vec![0; 32],
                })),
            }
            .encode_to_vec();

            let result = platform.query(QUERY_PATH, &request, version);
            let validation_result = result.unwrap();
            let response = GetIdentityContractNonceResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            assert!(matches!(
                extract_single_variant_or_panic!(
                    response.version.expect("expected a versioned response"),
                    get_identity_contract_nonce_response::Version::V0(inner),
                    inner
                )
                .result
                .unwrap(),
                get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(_)
            ));
        }
    }

    mod identity_balance_and_revision {
        use crate::error::query::QueryError;
        use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::{
            GetIdentityBalanceAndRevisionRequestV0, Version,
        };
        use dapi_grpc::platform::v0::{
            get_identity_balance_and_revision_response, GetIdentityBalanceAndRevisionRequest,
            GetIdentityBalanceAndRevisionResponse,
        };
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(response.version.expect("expected a versioned response"), get_identity_balance_and_revision_response::Version::V0(inner), inner).result.unwrap(),
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
        use dapi_grpc::Message;
        use drive::error::query::QuerySyntaxError;

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
                extract_single_variant_or_panic!(
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
                extract_single_variant_or_panic!(
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
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(
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
        use dapi_grpc::Message;

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
            ) = extract_single_variant_or_panic!(
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
                extract_single_variant_or_panic!(
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
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(response.version.expect("expected a versioned response"), get_data_contract_history_response::Version::V0(inner), inner).result.unwrap(),
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
        use dapi_grpc::Message;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::platform_value::string_encoding::Encoding;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use drive::error::query::QuerySyntaxError;

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

            let Some(result) = extract_single_variant_or_panic!(
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
                extract_single_variant_or_panic!(
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
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(response.version.expect("expected a versioned response"), get_identity_by_public_key_hash_response::Version::V0(inner), inner).result.unwrap(),
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
        use dapi_grpc::Message;

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
                extract_single_variant_or_panic!(response.version.expect("expected a versioned response"), get_identities_by_public_key_hashes_response::Version::V0(inner), inner).result.expect("expected a versioned result")
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
                extract_single_variant_or_panic!(response.version.expect("expected a versioned response"), get_identities_by_public_key_hashes_response::Version::V0(inner), inner).result.unwrap(),
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
        use dapi_grpc::platform::v0::{GetProofsRequest, GetProofsResponse};
        use dapi_grpc::platform::VersionedGrpcResponse;
        use dapi_grpc::Message;

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

            let proof = response
                .proof()
                .expect("expected a proof in versioned response");

            assert!(!proof.grovedb_proof.is_empty())
        }
    }

    mod version_upgrade_state {
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_request::{
            GetProtocolVersionUpgradeStateRequestV0, Version,
        };
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::get_protocol_version_upgrade_state_response_v0;
        use dapi_grpc::platform::v0::{
            get_protocol_version_upgrade_state_response, GetProtocolVersionUpgradeStateRequest,
            GetProtocolVersionUpgradeStateResponse,
        };
        use dapi_grpc::Message;
        use drive::drive::grove_operations::BatchInsertApplyType;
        use drive::drive::object_size_info::PathKeyElementInfo;
        use drive::drive::protocol_upgrade::{
            desired_version_for_validators_path, versions_counter_path, versions_counter_path_vec,
        };
        use drive::drive::Drive;
        use drive::grovedb::{Element, PathQuery, Query, QueryItem};
        use drive::query::GroveDb;
        use integer_encoding::VarInt;
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        use std::ops::RangeFull;

        const PATH: &str = "/versionUpgrade/state";

        #[test]
        fn test_query_empty_upgrade_state() {
            let (platform, version) = super::setup_platform();

            let request = GetProtocolVersionUpgradeStateRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeStateRequestV0 {
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeStateResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_state_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let versions = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_state_response_v0::Result::Versions(inner),
                inner
            );

            // we just started chain, there should be no versions

            assert!(versions.versions.is_empty())
        }

        #[test]
        fn test_query_upgrade_state() {
            let (platform, version) = super::setup_platform();

            let mut rand = StdRng::seed_from_u64(10);

            let drive = &platform.drive;

            let mut cache = drive.cache.write().unwrap();
            let version_counter = &mut cache.protocol_versions_counter;

            let transaction = drive.grove.start_transaction();

            version_counter
                .load_if_needed(drive, Some(&transaction), &version.drive)
                .expect("expected to load version counter");

            let path = desired_version_for_validators_path();
            let version_bytes = version.protocol_version.encode_var_vec();
            let version_element = Element::new_item(version_bytes.clone());

            let validator_pro_tx_hash: [u8; 32] = rand.gen();

            let mut operations = vec![];
            drive
                .batch_insert_if_changed_value(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        path,
                        validator_pro_tx_hash.as_slice(),
                        version_element,
                    )),
                    BatchInsertApplyType::StatefulBatchInsert,
                    Some(&transaction),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            let mut version_count = version_counter
                .get(&version.protocol_version)
                .cloned()
                .unwrap_or_default();

            version_count += 1;

            version_counter.set_block_cache_version_count(version.protocol_version, version_count); // push to block_cache

            drive
                .batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        versions_counter_path(),
                        version_bytes.as_slice(),
                        Element::new_item(version_count.encode_var_vec()),
                    )),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &version.drive,
                )
                .expect("expected to apply operations");

            drive
                .commit_transaction(transaction, &version.drive)
                .expect("expected to commit");

            cache.protocol_versions_counter.merge_block_cache();

            drop(cache);

            let request = GetProtocolVersionUpgradeStateRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeStateRequestV0 {
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeStateResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_state_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let versions = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_state_response_v0::Result::Versions(inner),
                inner
            );

            assert_eq!(versions.versions.len(), 1)
        }

        #[test]
        fn test_prove_empty_upgrade_state() {
            let (platform, version) = super::setup_platform();

            let request = GetProtocolVersionUpgradeStateRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeStateRequestV0 {
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeStateResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_state_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let proof = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_state_response_v0::Result::Proof(inner),
                inner
            );

            let path_query = PathQuery::new_unsized(
                versions_counter_path_vec(),
                Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
            );

            let elements = GroveDb::verify_query(proof.grovedb_proof.as_slice(), &path_query)
                .expect("expected to be able to verify query")
                .1;

            // we just started chain, there should be no versions

            assert!(elements.is_empty())
        }

        #[test]
        fn test_prove_upgrade_state() {
            let (platform, version) = super::setup_platform();

            let mut rand = StdRng::seed_from_u64(10);

            let drive = &platform.drive;

            let mut cache = drive.cache.write().unwrap();
            let version_counter = &mut cache.protocol_versions_counter;

            let transaction = drive.grove.start_transaction();

            version_counter
                .load_if_needed(drive, Some(&transaction), &version.drive)
                .expect("expected to load version counter");

            let path = desired_version_for_validators_path();
            let version_bytes = version.protocol_version.encode_var_vec();
            let version_element = Element::new_item(version_bytes.clone());

            let validator_pro_tx_hash: [u8; 32] = rand.gen();

            let mut operations = vec![];
            drive
                .batch_insert_if_changed_value(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        path,
                        validator_pro_tx_hash.as_slice(),
                        version_element,
                    )),
                    BatchInsertApplyType::StatefulBatchInsert,
                    Some(&transaction),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            let mut version_count = version_counter
                .get(&version.protocol_version)
                .cloned()
                .unwrap_or_default();

            version_count += 1;

            version_counter.set_block_cache_version_count(version.protocol_version, version_count); // push to block_cache

            drive
                .batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        versions_counter_path(),
                        version_bytes.as_slice(),
                        Element::new_item(version_count.encode_var_vec()),
                    )),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &version.drive,
                )
                .expect("expected to apply operations");

            drive
                .commit_transaction(transaction, &version.drive)
                .expect("expected to commit");

            cache.protocol_versions_counter.merge_block_cache();

            drop(cache);

            let request = GetProtocolVersionUpgradeStateRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeStateRequestV0 {
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeStateResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_state_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let proof = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_state_response_v0::Result::Proof(inner),
                inner
            );

            let path_query = PathQuery::new_unsized(
                versions_counter_path_vec(),
                Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
            );

            let elements = GroveDb::verify_query(proof.grovedb_proof.as_slice(), &path_query)
                .expect("expected to be able to verify query")
                .1;

            // we just started chain, there should be no versions

            assert_eq!(elements.len(), 1);

            let (_, _, element) = elements.first().unwrap();

            assert!(element.is_some());

            let element = element.clone().unwrap();

            let count_bytes = element.as_item_bytes().expect("expected item bytes");

            let count = u16::decode_var(count_bytes)
                .expect("expected to decode var int")
                .0;

            assert_eq!(count, 1);

            let upgrade = Drive::verify_upgrade_state(proof.grovedb_proof.as_slice(), version)
                .expect("expected to verify the upgrade counts")
                .1;

            assert_eq!(upgrade.len(), 1);
            assert_eq!(upgrade.get(&1), Some(1).as_ref());
        }
    }

    mod version_upgrade_vote_status {
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::Version;

        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0;
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0;
        use dapi_grpc::platform::v0::{
            get_protocol_version_upgrade_vote_status_response,
            GetProtocolVersionUpgradeVoteStatusRequest,
            GetProtocolVersionUpgradeVoteStatusResponse,
        };
        use dapi_grpc::Message;
        use drive::drive::grove_operations::BatchInsertApplyType;
        use drive::drive::object_size_info::PathKeyElementInfo;
        use drive::drive::protocol_upgrade::{
            desired_version_for_validators_path, desired_version_for_validators_path_vec,
            versions_counter_path,
        };
        use drive::drive::Drive;
        use drive::grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery};
        use drive::query::GroveDb;
        use integer_encoding::VarInt;
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        const PATH: &str = "/versionUpgrade/voteStatus";

        #[test]
        fn test_query_empty_upgrade_vote_status() {
            let (platform, version) = super::setup_platform();

            let mut rand = StdRng::seed_from_u64(10);

            let validator_pro_tx_hash: [u8; 32] = rand.gen();

            let request = GetProtocolVersionUpgradeVoteStatusRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusRequestV0 {
                    start_pro_tx_hash: validator_pro_tx_hash.to_vec(),
                    count: 5,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeVoteStatusResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_vote_status_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let version_signals = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::Result::Versions(inner),
                inner
            );

            // we just started chain, there should be no versions

            assert!(version_signals.version_signals.is_empty())
        }

        #[test]
        fn test_query_upgrade_vote_status() {
            let (platform, version) = super::setup_platform();

            let mut rand = StdRng::seed_from_u64(10);

            let validator_pro_tx_hash: [u8; 32] = rand.gen();

            let drive = &platform.drive;

            let mut cache = drive.cache.write().unwrap();
            let version_counter = &mut cache.protocol_versions_counter;

            let transaction = drive.grove.start_transaction();

            version_counter
                .load_if_needed(drive, Some(&transaction), &version.drive)
                .expect("expected to load version counter");

            let path = desired_version_for_validators_path();
            let version_bytes = version.protocol_version.encode_var_vec();
            let version_element = Element::new_item(version_bytes.clone());

            let mut operations = vec![];
            drive
                .batch_insert_if_changed_value(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        path,
                        validator_pro_tx_hash.as_slice(),
                        version_element,
                    )),
                    BatchInsertApplyType::StatefulBatchInsert,
                    Some(&transaction),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            let mut version_count = version_counter
                .get(&version.protocol_version)
                .cloned()
                .unwrap_or_default();

            version_count += 1;

            version_counter.set_block_cache_version_count(version.protocol_version, version_count); // push to block_cache

            drive
                .batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        versions_counter_path(),
                        version_bytes.as_slice(),
                        Element::new_item(version_count.encode_var_vec()),
                    )),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &version.drive,
                )
                .expect("expected to apply operations");

            drive
                .commit_transaction(transaction, &version.drive)
                .expect("expected to commit");

            cache.protocol_versions_counter.merge_block_cache();

            drop(cache);

            let request = GetProtocolVersionUpgradeVoteStatusRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusRequestV0 {
                    start_pro_tx_hash: validator_pro_tx_hash.to_vec(),
                    count: 5,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeVoteStatusResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_vote_status_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let version_signals = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_vote_status_response::get_protocol_version_upgrade_vote_status_response_v0::Result::Versions(inner),
                inner
            );

            assert_eq!(version_signals.version_signals.len(), 1)
        }

        #[test]
        fn test_prove_empty_upgrade_vote_status() {
            let (platform, version) = super::setup_platform();

            let mut rand = StdRng::seed_from_u64(10);

            let validator_pro_tx_hash: [u8; 32] = rand.gen();

            let request = GetProtocolVersionUpgradeVoteStatusRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusRequestV0 {
                    start_pro_tx_hash: validator_pro_tx_hash.to_vec(),
                    count: 5,
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeVoteStatusResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_vote_status_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let proof = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_vote_status_response_v0::Result::Proof(inner),
                inner
            );

            let path = desired_version_for_validators_path_vec();

            let query_item = QueryItem::RangeFrom(validator_pro_tx_hash.to_vec()..);

            let path_query = PathQuery::new(
                path,
                SizedQuery::new(Query::new_single_query_item(query_item), Some(5), None),
            );

            let elements = GroveDb::verify_query(proof.grovedb_proof.as_slice(), &path_query)
                .expect("expected to be able to verify query")
                .1;

            // we just started chain, there should be no versions

            assert!(elements.is_empty())
        }

        #[test]
        fn test_prove_upgrade_vote_status() {
            let (platform, version) = super::setup_platform();

            let mut rand = StdRng::seed_from_u64(10);

            let drive = &platform.drive;

            let mut cache = drive.cache.write().unwrap();
            let version_counter = &mut cache.protocol_versions_counter;

            let transaction = drive.grove.start_transaction();

            version_counter
                .load_if_needed(drive, Some(&transaction), &version.drive)
                .expect("expected to load version counter");

            let path = desired_version_for_validators_path();
            let version_bytes = version.protocol_version.encode_var_vec();
            let version_element = Element::new_item(version_bytes.clone());

            let validator_pro_tx_hash: [u8; 32] = rand.gen();

            let mut operations = vec![];
            drive
                .batch_insert_if_changed_value(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        path,
                        validator_pro_tx_hash.as_slice(),
                        version_element,
                    )),
                    BatchInsertApplyType::StatefulBatchInsert,
                    Some(&transaction),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            let mut version_count = version_counter
                .get(&version.protocol_version)
                .cloned()
                .unwrap_or_default();

            version_count += 1;

            version_counter.set_block_cache_version_count(version.protocol_version, version_count); // push to block_cache

            drive
                .batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement((
                        versions_counter_path(),
                        version_bytes.as_slice(),
                        Element::new_item(version_count.encode_var_vec()),
                    )),
                    &mut operations,
                    &version.drive,
                )
                .expect("expected batch to insert");

            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    operations,
                    &mut vec![],
                    &version.drive,
                )
                .expect("expected to apply operations");

            drive
                .commit_transaction(transaction, &version.drive)
                .expect("expected to commit");

            cache.protocol_versions_counter.merge_block_cache();

            drop(cache);

            let request = GetProtocolVersionUpgradeVoteStatusRequest {
                version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusRequestV0 {
                    start_pro_tx_hash: validator_pro_tx_hash.to_vec(),
                    count: 5,
                    prove: true,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetProtocolVersionUpgradeVoteStatusResponse::decode(
                validation_result.data.unwrap().as_slice(),
            )
            .unwrap();

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_protocol_version_upgrade_vote_status_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let proof = extract_variant_or_panic!(
                result,
                get_protocol_version_upgrade_vote_status_response_v0::Result::Proof(inner),
                inner
            );

            let path = desired_version_for_validators_path_vec();

            let query_item = QueryItem::RangeFrom(validator_pro_tx_hash.to_vec()..);

            let path_query = PathQuery::new(
                path,
                SizedQuery::new(Query::new_single_query_item(query_item), Some(5), None),
            );

            let elements = GroveDb::verify_query(proof.grovedb_proof.as_slice(), &path_query)
                .expect("expected to be able to verify query")
                .1;

            // we just started chain, there should be no versions

            assert_eq!(elements.len(), 1);

            let (_, _, element) = elements.first().unwrap();

            assert!(element.is_some());

            let element = element.clone().unwrap();

            let count_bytes = element.as_item_bytes().expect("expected item bytes");

            let count = u16::decode_var(count_bytes)
                .expect("expected to decode var int")
                .0;

            assert_eq!(count, 1);

            let upgrade = Drive::verify_upgrade_vote_status(
                proof.grovedb_proof.as_slice(),
                Some(validator_pro_tx_hash),
                5,
                version,
            )
            .expect("expected to verify the upgrade counts")
            .1;

            assert_eq!(upgrade.len(), 1);
            assert_eq!(upgrade.get(&validator_pro_tx_hash), Some(1).as_ref());
        }
    }

    mod epoch_infos {
        use dapi_grpc::platform::v0::get_epochs_info_request::{GetEpochsInfoRequestV0, Version};
        use dapi_grpc::platform::v0::{
            get_epochs_info_response, GetEpochsInfoRequest, GetEpochsInfoResponse,
        };
        use dapi_grpc::Message;

        const PATH: &str = "/epochInfos";

        #[test]
        fn test_query_empty_epoch_infos() {
            let (platform, version) = super::setup_platform();

            let request = GetEpochsInfoRequest {
                version: Some(Version::V0(GetEpochsInfoRequestV0 {
                    start_epoch: None, // 0
                    count: 5,
                    ascending: true,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetEpochsInfoResponse::decode(
                validation_result.data.expect("expected data").as_slice(),
            )
            .expect("expected to decode response");

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_epochs_info_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let epoch_infos = extract_variant_or_panic!(
                result,
                get_epochs_info_response::get_epochs_info_response_v0::Result::Epochs(inner),
                inner
            );

            // we just started chain, there should be only 1 epoch info for the first epoch

            assert!(epoch_infos.epoch_infos.is_empty())
        }

        #[test]
        fn test_query_empty_epoch_infos_descending() {
            let (platform, version) = super::setup_platform();

            let request = GetEpochsInfoRequest {
                version: Some(Version::V0(GetEpochsInfoRequestV0 {
                    start_epoch: None, // 0
                    count: 5,
                    ascending: false,
                    prove: false,
                })),
            }
            .encode_to_vec();

            let validation_result = platform
                .query(PATH, &request, version)
                .expect("expected query to succeed");
            let response = GetEpochsInfoResponse::decode(
                validation_result.data.expect("expected data").as_slice(),
            )
            .expect("expected to decode response");

            let result = extract_single_variant_or_panic!(
                response.version.expect("expected a versioned response"),
                get_epochs_info_response::Version::V0(inner),
                inner
            )
            .result
            .expect("expected a result");

            let epoch_infos = extract_variant_or_panic!(
                result,
                get_epochs_info_response::get_epochs_info_response_v0::Result::Epochs(inner),
                inner
            );

            // we just started chain, there should be only 1 epoch info for the first epoch

            assert!(epoch_infos.epoch_infos.is_empty())
        }
    }
}
