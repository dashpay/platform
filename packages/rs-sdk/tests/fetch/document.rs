//! Test document CRUDL operations

use std::sync::Arc;

use super::{common::setup_logs, config::Config};
use dash_sdk::platform::{DocumentQuery, Fetch, FetchMany};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::{DataContract, Identifier};
use drive::query::{DriveQuery, OrderClause, WhereClause};

/// Given some data contract ID, document type and document ID, when I fetch it, then I get it.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("document_read").await;

    let data_contract_id = cfg.existing_data_contract_id;

    let contract = Arc::new(
        DataContract::fetch(&sdk, data_contract_id)
            .await
            .expect("fetch data contract")
            .expect("data contract not found"),
    );

    // Fetch multiple documents so that we get document ID
    let all_docs_query =
        DocumentQuery::new(Arc::clone(&contract), &cfg.existing_document_type_name)
            .expect("create SdkDocumentQuery");
    let first_doc = Document::fetch_many(&sdk, all_docs_query)
        .await
        .expect("fetch many documents")
        .pop_first()
        .expect("first item must exist")
        .1
        .expect("document must exist");

    // Now query for individual document
    let query = DocumentQuery::new(contract, &cfg.existing_document_type_name)
        .expect("create SdkDocumentQuery")
        .with_document_id(&first_doc.id());

    let doc = Document::fetch(&sdk, query)
        .await
        .expect("fetch document")
        .expect("document must be found");

    assert_eq!(first_doc, doc);
}

/// Given some non-existing data contract ID, when I create [DocumentQuery], I get an error.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read_no_contract() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("document_read_no_contract").await;

    let data_contract_id = Identifier::from_bytes(&[0; 32]).expect("create Identifier");

    let query = DocumentQuery::new_with_data_contract_id(
        &sdk,
        data_contract_id,
        &cfg.existing_document_type_name,
    )
    .await;

    assert!(matches!(
        query,
        Err(dash_sdk::error::Error::MissingDependency(_, _))
    ));
}

/// Given some data contract ID, document type and non-existing document ID, when I fetch it, I get zero documents but
/// no error.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_read_no_document() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("document_read_no_document").await;

    let data_contract_id = cfg.existing_data_contract_id;
    let document_id = [0; 32].into();

    let query = DocumentQuery::new_with_data_contract_id(
        &sdk,
        data_contract_id,
        &cfg.existing_document_type_name,
    )
    .await
    .expect("create SdkDocumentQuery")
    .with_document_id(&document_id);

    let doc = Document::fetch(&sdk, query).await.expect("fetch document");

    assert!(doc.is_none(), "document must not be found");
}

/// Given some data contract ID and document type with at least one document, when I fetch many documents using DriveQuery
/// as a query, then I get one or more items.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_list_drive_query() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("document_list_drive_query").await;

    let data_contract_id = cfg.existing_data_contract_id;

    let data_contract = DataContract::fetch(&sdk, data_contract_id)
        .await
        .expect("fetch data contract")
        .expect("data contract not found");

    let doctype = data_contract
        .document_type_for_name(&cfg.existing_document_type_name)
        .expect("document type not found");

    let query = DriveQuery {
        contract: &data_contract,
        document_type: doctype,
        internal_clauses: Default::default(),
        offset: None,
        limit: Some(1),
        order_by: Default::default(),
        start_at: None,
        start_at_included: true,
        block_time_ms: None,
    };

    let docs = <Document>::fetch_many(&sdk, query)
        .await
        .expect("fetch many documents");

    assert!(!docs.is_empty());
    let doc_ids: Vec<String> = docs
        .iter()
        .map(|d| d.0.to_string(Encoding::Base64))
        .collect();

    tracing::info!(documents=?doc_ids, "fetched documents");
}

/// Given some data contract ID and document type with at least one document, when I list documents using DocumentQuery
/// as a query, then I get one or more items.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn document_list_document_query() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("document_list_document_query").await;

    let data_contract_id = cfg.existing_data_contract_id;

    let data_contract = Arc::new(
        DataContract::fetch(&sdk, data_contract_id)
            .await
            .expect("fetch data contract")
            .expect("data contra)ct not found"),
    );

    let query = DocumentQuery::new(Arc::clone(&data_contract), &cfg.existing_document_type_name)
        .expect("document query created");

    let docs = <Document>::fetch_many(&sdk, query)
        .await
        .expect("fetch many documents");

    assert!(!docs.is_empty());
    let doc_ids: Vec<String> = docs
        .iter()
        .map(|d| d.0.to_string(Encoding::Base64))
        .collect();

    tracing::info!(documents=?doc_ids, "fetched documents");
}

/// Check if bug when processing Where clause containing `Value::Text("l")` string is fixed.
///
///query: storage: protocol: value error: structure error: value was a string, but could not be decoded from base 58
/// Given document query with where clause contains to `"l"` string,
/// when I fetch documents,
/// then I don't get error:
///     `query: storage: protocol: value error: structure error: value was a string, but could not be decoded from base 58`.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[allow(non_snake_case)]
async fn document_list_bug_value_text_decode_base58_PLAN_653() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("document_list_bug_value_text_decode_base58")
        .await;

    let data_contract_id = cfg.existing_data_contract_id;

    let contract = Arc::new(
        DataContract::fetch(&sdk, data_contract_id)
            .await
            .expect("fetch data contract")
            .expect("data contract not found"),
    );

    // Fetch multiple documents so that we get document ID
    let query = DocumentQuery::new(Arc::clone(&contract), &cfg.existing_document_type_name)
        .expect("create SdkDocumentQuery")
        .with_where(WhereClause {
            field: "records.dashUniqueIdentityId".to_string(),
            operator: drive::query::WhereOperator::Equal,
            value: dpp::platform_value::Value::Text("l".into()),
        })
        .with_order_by(OrderClause {
            field: "records.dashUniqueIdentityId".to_string(),
            ascending: true,
        });

    let _docs = Document::fetch_many(&sdk, query)
        .await
        .expect("fetch many documents");
}
