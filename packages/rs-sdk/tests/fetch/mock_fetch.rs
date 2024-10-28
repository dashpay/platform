//! Tests of mocked Fetch trait implementations.

use super::common::{mock_data_contract, mock_document_type, setup_logs};
use dapi_grpc::platform::v0::{
    get_data_contract_response::GetDataContractResponseV0, GetDataContractRequest,
    GetDataContractResponse, Proof, ResponseMetadata,
};
use dash_sdk::{
    error::StaleNodeError,
    platform::{DocumentQuery, Fetch, Query},
    Sdk,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
    },
    document::Document,
    identity::{accessors::IdentityGettersV0, IdentityV0},
    prelude::{DataContract, Identifier, Identity},
    version::PlatformVersion,
};
use rs_dapi_client::mock::InnerInto;

#[tokio::test]
/// Given some identity, when I fetch it using mock API, then I get the same identity
async fn test_mock_fetch_identity() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let query = expected.id();

    sdk.mock()
        .expect_fetch(query, Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = Identity::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// When I define mock expectation twice for the same request, second call ends with error
async fn test_mock_fetch_duplicate_expectation() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let expected2 =
        Identity::random_identity(3, Some(2), PlatformVersion::latest()).expect("random identity");

    let query = expected.id();

    sdk.mock()
        .expect_fetch(query, Some(expected.clone()))
        .await
        .expect("first expectation should be added correctly");

    sdk.mock()
        .expect_fetch(query, Some(expected2))
        .await
        .expect_err("conflicting expectation should fail");

    let retrieved = Identity::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
/// Given some random identity ID, when I fetch it using mock API, then I get None
async fn test_mock_fetch_identity_not_found() {
    let mut sdk = Sdk::new_mock();

    let id = Identifier::random();

    sdk.mock()
        .expect_fetch(id, None as Option<Identity>)
        .await
        .unwrap();

    let retrieved = Identity::fetch(&sdk, id)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}

/// Given some data contract, when I fetch it by ID, I get it.
#[tokio::test]
async fn test_mock_fetch_data_contract() {
    let mut sdk = Sdk::new_mock();

    let document_type: DocumentType = mock_document_type();
    let expected = mock_data_contract(Some(&document_type));
    let id = expected.id();

    sdk.mock()
        .expect_fetch(id, Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = DataContract::fetch(&sdk, id)
        .await
        .unwrap()
        .expect("object should exist");
    assert_eq!(retrieved, expected);
}

/// Given some data contract, document type name and document, when I fetch expected document using mock Sdk, I get it.
#[tokio::test]
async fn test_mock_fetch_document() {
    use dpp::document::DocumentV0Getters;

    let mut sdk = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected = document_type
        .random_document(None, sdk.version())
        .expect("document should be created");
    let document_id = expected.id();
    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_data_contract_id] will fetch the data contract first, so we need to define an expectation for it.
    sdk.mock()
        .expect_fetch(data_contract.id(), Some(data_contract.clone()))
        .await
        .unwrap();

    let query =
        DocumentQuery::new_with_data_contract_id(&sdk, data_contract.id(), document_type_name)
            .await
            .expect("create document query")
            .with_document_id(&document_id);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .unwrap();

    let retrieved = Document::fetch(&sdk, query)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
async fn test_mock_fetch_retry() {
    setup_logs();

    let mut sdk = Sdk::new_mock();
    let query_correct = Identifier::random();
    let query_stale = Identifier::random();
    assert_ne!(query_correct, query_stale);

    // CONFIGURE EXPECTATIONS
    let mut mock = sdk.mock();

    // On DAPI level, we mock some response that will indicate to the SDK that the node is behind and we should retry
    // First, we return height 10 to indicate current tip
    let request: GetDataContractRequest = query_correct.query(true).expect("create query");
    let metadata = ResponseMetadata {
        height: 10,
        ..Default::default()
    };
    let response: GetDataContractResponse = GetDataContractResponseV0 {
        metadata: Some(metadata.clone()),
        result: Some(
            dapi_grpc::platform::v0::get_data_contract_response::get_data_contract_response_v0::Result::Proof(
                Proof {
                    ..Default::default()
                },
            )),
    }.into();
    mock.expect_dapi(&request, &Ok(response.inner_into()))
        .await
        .expect("add mock 1");

    mock.expect_from_proof_with_metadata(
        &request,
        Ok((Option::<DataContract>::None, metadata, Default::default())),
    )
    .expect("add proof expectation 1");

    // Now, we return height 2 to indicate that the node is behind
    let request: GetDataContractRequest = query_stale.query(true).expect("create query");
    let metadata = ResponseMetadata {
        height: 2,
        ..Default::default()
    };
    let response: GetDataContractResponse = GetDataContractResponseV0 {
        metadata: Some(metadata.clone()),
        result: Some(
            dapi_grpc::platform::v0::get_data_contract_response::get_data_contract_response_v0::Result::Proof(
                Proof {
                    ..Default::default()
                },
            )),
    }.into();

    mock.expect_dapi(&request, &Ok(response.inner_into()))
        .await
        .expect("add mock 2");
    mock.expect_from_proof_with_metadata(
        &request,
        Ok((Option::<DataContract>::None, metadata, Default::default())),
    )
    .expect("add proof expectation 2");

    // EXECUTE THE TEST
    drop(mock);

    DataContract::fetch(&sdk, query_correct)
        .await
        .expect("first fetch should succeed");

    let e = DataContract::fetch(&sdk, query_stale)
        .await
        .expect_err("second fetch should fail");

    match e {
        dash_sdk::Error::StaleNode(StaleNodeError::Height {
            expected_height,
            received_height,
            tolerance_blocks,
        }) => {
            assert_eq!(expected_height, 10);
            assert_eq!(received_height, 2);
            assert_eq!(tolerance_blocks, 1);
        }
        _ => panic!("unexpected error: {:?}", e),
    }
}
