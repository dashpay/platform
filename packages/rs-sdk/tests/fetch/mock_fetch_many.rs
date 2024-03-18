use std::collections::BTreeMap;

use super::common::{mock_data_contract, mock_document_type};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
    },
    document::{Document, DocumentV0Getters},
};
use rs_sdk::{
    platform::{DocumentQuery, FetchMany},
    Sdk,
};

/// Given some data contract, document type and 1 document of this type, when I request multiple documents, I get that
/// document.
#[tokio::test]
async fn test_mock_document_fetch_many() {
    let sdk = Sdk::new_mock();
    let document_type: DocumentType = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));

    let expected_doc = document_type
        .random_document(None, sdk.version())
        .expect("document should be created");
    let expected = BTreeMap::from([(expected_doc.id(), Some(expected_doc.clone()))]);

    // document that should not be returned, as it will be defined as a duplicate
    let not_expected_doc = document_type
        .random_document(None, sdk.version())
        .expect("document 2 should be created");
    let not_expected = BTreeMap::from([(not_expected_doc.id(), Some(not_expected_doc))]);

    let document_type_name = document_type.name();

    // [DocumentQuery::new_with_document_id] will fetch the data contract first, so we need to define an expectation for it.
    sdk.mock()
        .expect_fetch(data_contract.id(), Some(data_contract.clone()))
        .await
        .unwrap();

    let query =
        DocumentQuery::new(data_contract, document_type_name).expect("create document query");
    sdk.mock()
        .expect_fetch_many(query.clone(), Some(expected.clone()))
        .await
        .unwrap();

    sdk.mock()
        .expect_fetch_many(query.clone(), Some(not_expected))
        .await
        .expect_err("duplicate expectations are not allowed");

    let retrieved = Document::fetch_many(&sdk, query).await.unwrap();

    assert!(!retrieved.is_empty());
    assert_eq!(retrieved, expected);
}
