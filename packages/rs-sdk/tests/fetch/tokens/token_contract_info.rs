use crate::fetch::common::setup_logs;
use dash_sdk::platform::tokens::token_contract_info::TokenContractInfoQuery;
use dash_sdk::platform::{Fetch, Identifier, Query};
use dash_sdk::Sdk;
use dpp::tokens::contract_info::TokenContractInfo;

#[tokio::test]
async fn test_token_contract_info_fetch_by_identifier() {
    setup_logs();

    let mut sdk = Sdk::new_mock();
    let token_id = Identifier::from_bytes(&[1u8; 32]).unwrap();

    // Set up mock expectation
    sdk.mock()
        .expect_fetch(token_id, None as Option<TokenContractInfo>)
        .await
        .unwrap();

    // Test fetching by identifier directly
    let result = TokenContractInfo::fetch_by_identifier(&sdk, token_id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // Mock returns None as expected
}

#[tokio::test]
async fn test_token_contract_info_fetch_with_query() {
    setup_logs();

    let mut sdk = Sdk::new_mock();
    let token_id = Identifier::from_bytes(&[2u8; 32]).unwrap();

    let query = TokenContractInfoQuery { token_id };

    // Set up mock expectation
    sdk.mock()
        .expect_fetch(query.clone(), None as Option<TokenContractInfo>)
        .await
        .unwrap();

    // Test fetching with explicit query
    let result = TokenContractInfo::fetch(&sdk, query).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // Mock returns None as expected
}

#[tokio::test]
async fn test_token_contract_info_query_prove_true() {
    let token_id = Identifier::from_bytes(&[3u8; 32]).unwrap();
    let query = TokenContractInfoQuery { token_id };

    let request = query.query(true).unwrap();

    match request.version.unwrap() {
        dapi_grpc::platform::v0::get_token_contract_info_request::Version::V0(v0) => {
            assert_eq!(v0.token_id, token_id.to_vec());
            assert!(v0.prove);
        }
    }
}

#[tokio::test]
async fn test_token_contract_info_query_prove_false() {
    let token_id = Identifier::from_bytes(&[4u8; 32]).unwrap();
    let query = TokenContractInfoQuery { token_id };

    let request = query.query(false).unwrap();

    match request.version.unwrap() {
        dapi_grpc::platform::v0::get_token_contract_info_request::Version::V0(v0) => {
            assert_eq!(v0.token_id, token_id.to_vec());
            assert!(!v0.prove);
        }
    }
}
