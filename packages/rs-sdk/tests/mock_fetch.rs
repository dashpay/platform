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
    platform_value::platform_value,
    prelude::{DataContract, Identity},
};
use rs_sdk::{
    platform::{DocumentQuery, Fetch},
    Sdk,
};

include!("common.rs");

#[tokio::test]
/// Given some identity ID, when I fetch it using mock API, then I get the same identity
async fn test_mock_fetch_identity() {
    let mut api = Sdk::new_mock();

    let expected: Identity = Identity::from(IdentityV0::default());
    let id = expected.id();

    api.mock().expect_fetch(id, expected.clone()).await;

    let retrieved = dpp::prelude::Identity::fetch(&mut api, id)
        .await
        .unwrap()
        .expect("object should exist");

    assert_eq!(retrieved, expected);
}

#[tokio::test]
async fn test_mock_fetch_data_contract() {
    let mut api = Sdk::new_mock();

    let expected = mock_data_contract(None);
    let id = expected.id();

    api.mock().expect_fetch(id, expected.clone()).await;

    let retrieved = DataContract::fetch(&mut api, id)
        .await
        .unwrap()
        .expect("object should exist");
    assert_eq!(retrieved, expected);
}

#[tokio::test]
async fn test_mock_fetch_document() {
    use dpp::document::DocumentV0Getters;

    let mut api = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected = document_type
        .random_filled_document(None, Sdk::version())
        .expect("document should be created");
    let document_id = expected.id();
    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_document_id] will fetch the data contract first, so we need to define an expectation for it.
    api.mock()
        .expect_fetch(data_contract.id(), data_contract.clone())
        .await;

    let query = DocumentQuery::new_with_document_id(
        &mut api,
        data_contract.id(),
        document_type_name,
        document_id,
    )
    .await
    .expect("create document query");

    api.mock()
        .expect_fetch(query.clone(), expected.clone())
        .await;

    let retrieved = Document::fetch(&mut api, query)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(retrieved, expected);
}
