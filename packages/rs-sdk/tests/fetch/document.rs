//! Test document CRUDL operations

use std::sync::Arc;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::{DataContract, Identifier};
use drive::query::DriveQuery;

use crate::common::{
    base64_identifier, setup_api, setup_logs, DATA_CONTRACT_ID, DOCUMENT_TYPE_NAME,
};
use rs_sdk::platform::DocumentQuery;
use rs_sdk::platform::{Fetch, List};

/// Given some data contract ID, document type and document ID, when I fetch it, then I get it.
///
/// This test is ignored because it requires a running Platform. To run it, set constants in `common.rs` and run:
///
/// ```bash
/// cargo test -p rs-sdk -- --ignored
/// ```
#[ignore = "needs access to running Dash Platform network"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read() {
    setup_logs();

    let mut sdk = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);

    let contract = Arc::new(
        DataContract::fetch(&mut sdk, data_contract_id)
            .await
            .expect("fetch data contract")
            .expect("data contract not found"),
    );

    // List documents so that we get document ID
    let all_docs_query = DocumentQuery::new(Arc::clone(&contract), DOCUMENT_TYPE_NAME)
        .expect("create SdkDocumentQuery");
    let docs = Document::list(&mut sdk, all_docs_query)
        .await
        .expect("list documents")
        .expect("no documents found");
    let first_doc = docs.first().expect("document must exist");

    // Now query for individual document
    let query = DocumentQuery::new(contract, DOCUMENT_TYPE_NAME)
        .expect("create SdkDocumentQuery")
        .with_document_id(&first_doc.id());

    let doc = Document::fetch(&mut sdk, query)
        .await
        .expect("fetch document")
        .expect("document must be found");

    assert_eq!(first_doc, &doc);
}

/// Given some non-existing data contract ID, when I create [DocumentQuery], I get an error.
///
/// This test is ignored because it requires a running Platform. To run it, set constants in `common.rs` and run:
///
/// ```bash
/// cargo test -p rs-sdk -- --ignored
/// ```
#[ignore = "needs access to running Dash Platform network"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read_no_contract() {
    setup_logs();

    let mut sdk = setup_api();

    let data_contract_id = Identifier::from_bytes(&[0; 32]).expect("create Identifier");

    let query =
        DocumentQuery::new_with_data_contract_id(&mut sdk, data_contract_id, DOCUMENT_TYPE_NAME)
            .await;

    assert!(matches!(
        query,
        Err(rs_sdk::error::Error::MissingDependency(_, _))
    ));
}

/// Given some data contract ID, document type and non-existing document ID, when I fetch it, I get zero documents but
/// no error.
///
/// This test is ignored because it requires a running Platform. To run it, set constants in `common.rs` and run:
///
/// ```bash
/// cargo test -p rs-sdk -- --ignored
/// ```
#[ignore = "needs access to running Dash Platform network"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read_no_document() {
    setup_logs();

    let mut sdk = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);
    let document_id = Identifier::from_bytes(&[0; 32]).expect("create Identifier");

    let query =
        DocumentQuery::new_with_data_contract_id(&mut sdk, data_contract_id, DOCUMENT_TYPE_NAME)
            .await
            .expect("create SdkDocumentQuery")
            .with_document_id(&document_id);

    let doc = Document::fetch(&mut sdk, query)
        .await
        .expect("fetch document");

    assert!(doc.is_none(), "document must not be found");
}

/// Given some data contract ID and document type with at least one document, when I list documents using DriveQuery
/// as a query, then I get one or more items.
///
/// This test is ignored because it requires a running Platform. To run it, set constants in `common.rs` and run:
///
/// ```bash
/// cargo test -p rs-sdk -- --ignored
/// ```
#[ignore = "needs access to running Dash Platform network"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_list_drive_query() {
    setup_logs();

    let mut sdk = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);

    let data_contract = DataContract::fetch(&mut sdk, data_contract_id)
        .await
        .expect("fetch data contract")
        .expect("data contract not found");

    let doctype = data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("document type not found");

    let query = DriveQuery::any_item_query(&data_contract, doctype);

    let docs = <Document>::list(&mut sdk, query)
        .await
        .expect("list documents")
        .expect("no documents found");

    assert!(docs.len() > 0);
    let doc_ids: Vec<String> = docs
        .iter()
        .map(|d| d.id().to_string(Encoding::Base64))
        .collect();

    tracing::info!(documents=?doc_ids, "fetched documents");
}

/// Given some data contract ID and document type with at least one document, when I list documents using DocumentQuery
/// as a query, then I get one or more items.
///
/// This test is ignored because it requires a running Platform. To run it, set constants in `common.rs` and run:
///
/// ```bash
/// cargo test -p rs-sdk -- --ignored
/// ```
#[ignore = "needs access to running Dash Platform network"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_list_document_query() {
    setup_logs();

    let mut sdk = setup_api();

    let data_contract_id = base64_identifier(DATA_CONTRACT_ID);

    let data_contract = Arc::new(
        DataContract::fetch(&mut sdk, data_contract_id)
            .await
            .expect("fetch data contract")
            .expect("data contra)ct not found"),
    );

    let query = DocumentQuery::new(Arc::clone(&data_contract), DOCUMENT_TYPE_NAME)
        .expect("document query created");

    let docs = <Document>::list(&mut sdk, query)
        .await
        .expect("list documents")
        .expect("no documents found");

    assert!(docs.len() > 0);
    let doc_ids: Vec<String> = docs
        .iter()
        .map(|d| d.id().to_string(Encoding::Base64))
        .collect();

    tracing::info!(documents=?doc_ids, "fetched documents");
}
