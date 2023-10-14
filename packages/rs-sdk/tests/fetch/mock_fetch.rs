//!
//!

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
};
use rs_sdk::{
    platform::{DocumentQuery, Fetch},
    Sdk,
};
use crate::common::{mock_data_contract, mock_document_type};

#[tokio::test]
/// Given some identity, when I fetch it using mock API, then I get the same identity
async fn test_mock_fetch_identity() {
    let mut sdk = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let query = expected.id();

    sdk.mock().expect_fetch(query, Some(expected.clone())).await;

    let retrieved = Identity::fetch(&mut sdk, query)
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

    sdk.mock().expect_fetch(id, None as Option<Identity>).await;

    let retrieved = Identity::fetch(&mut sdk, id)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}

/// Given some data contract, when I fetch it by ID, I get it.
#[tokio::test]
async fn test_mock_fetch_data_contract() {
    let mut sdk = Sdk::new_mock();

    let expected = mock_data_contract(None);
    let id = expected.id();

    sdk.mock().expect_fetch(id, Some(expected.clone())).await;

    let retrieved = DataContract::fetch(&mut sdk, id)
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
        .await;

    let query =
        DocumentQuery::new_with_data_contract_id(&mut sdk, data_contract.id(), document_type_name)
            .await
            .expect("create document query")
            .with_document_id(&document_id);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await;

    let retrieved = Document::fetch(&mut sdk, query)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(retrieved, expected);
}
