//! Document operation tests

mod common;
use common::*;
use wasm_bindgen_test::*;
use wasm_sdk::{
    fetch::{fetch_documents, FetchOptions},
    fetch_unproved::fetch_documents_unproved,
    query::DocumentQuery,
    state_transitions::document::DocumentBatchBuilder,
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_document_query() {
    let contract_id = test_contract_id();
    let document_type = "message";

    let query = DocumentQuery::new(&contract_id, document_type);
    assert!(query.is_ok(), "Should create document query");

    let mut q = query.unwrap();

    // Test adding where clauses
    q.add_where_clause("author", "=", &test_identity_id().into());
    q.add_where_clause("timestamp", ">", &1234567890.into());

    // Test adding order by
    q.add_order_by("timestamp", false);

    // Test setting limit and offset
    q.set_limit(10);
    q.set_offset(5);

    // Verify query properties
    assert_eq!(q.contract_id(), contract_id);
    assert_eq!(q.document_type(), document_type);
    assert_eq!(q.limit(), Some(10));
    assert_eq!(q.offset(), Some(5));

    let where_clauses = q.get_where_clauses();
    assert!(where_clauses.is_ok(), "Should get where clauses");

    let order_by_clauses = q.get_order_by_clauses();
    assert!(order_by_clauses.is_ok(), "Should get order by clauses");
}

#[wasm_bindgen_test]
async fn test_document_batch_builder() {
    let owner_id = test_identity_id();
    let contract_id = test_contract_id();
    let document_type = "message";

    let builder = DocumentBatchBuilder::new(&owner_id);
    assert!(builder.is_ok(), "Should create document batch builder");

    let mut batch = builder.unwrap();

    // Test adding create document
    let create_data = js_sys::Object::new();
    js_sys::Reflect::set(&create_data, &"text".into(), &"Hello, World!".into()).unwrap();
    js_sys::Reflect::set(&create_data, &"timestamp".into(), &1234567890.into()).unwrap();

    let create_result = batch.add_create_document(
        &contract_id,
        document_type,
        &test_document_id(),
        create_data.into(),
    );
    assert!(create_result.is_ok(), "Should add create document");

    // Test adding delete document
    let delete_result = batch.add_delete_document(&contract_id, document_type, &test_document_id());
    assert!(delete_result.is_ok(), "Should add delete document");

    // Test adding replace document
    let replace_data = js_sys::Object::new();
    js_sys::Reflect::set(&replace_data, &"text".into(), &"Updated text".into()).unwrap();
    js_sys::Reflect::set(&replace_data, &"timestamp".into(), &1234567900.into()).unwrap();

    let replace_result = batch.add_replace_document(
        &contract_id,
        document_type,
        &test_document_id(),
        1,
        replace_data.into(),
    );
    assert!(replace_result.is_ok(), "Should add replace document");

    // Test building the batch
    let state_transition = batch.build(0);
    assert!(state_transition.is_ok(), "Should build document batch");
    assert!(
        !state_transition.unwrap().is_empty(),
        "State transition should not be empty"
    );
}

#[wasm_bindgen_test]
async fn test_fetch_documents() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    let document_type = "message";

    // Create a simple where clause
    let where_clause = js_sys::Object::new();

    // Test basic fetch
    let result =
        fetch_documents(&sdk, &contract_id, document_type, where_clause.into(), None).await;
    assert!(result.is_ok(), "Should fetch documents");

    // Test fetch with options
    let options = FetchOptions::new();
    let where_clause2 = js_sys::Object::new();
    let result_with_options = fetch_documents(
        &sdk,
        &contract_id,
        document_type,
        where_clause2.into(),
        Some(options),
    )
    .await;
    assert!(
        result_with_options.is_ok(),
        "Should fetch documents with options"
    );
}

#[wasm_bindgen_test]
async fn test_fetch_documents_unproved() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    let document_type = "message";

    let where_clause = js_sys::Object::new();
    let order_by = js_sys::Object::new();

    let result = fetch_documents_unproved(
        &sdk,
        &contract_id,
        document_type,
        where_clause.into(),
        order_by.into(),
        Some(10),
        None,
        None,
    )
    .await;
    assert!(result.is_ok(), "Should fetch documents without proof");
}

#[wasm_bindgen_test]
async fn test_document_transitions() {
    let owner_id = test_identity_id();
    let contract_id = test_contract_id();
    let document_type = "profile";

    // Test transfer document
    let transfer_result = wasm_sdk::state_transitions::document::transfer_document(
        &contract_id,
        document_type,
        &test_document_id(),
        &owner_id,
        &test_identity_id(), // recipient
        1,                   // revision
        1,                   // identity nonce
        0,                   // signature key id
    );
    assert!(
        transfer_result.is_ok(),
        "Should create transfer document transition"
    );

    // Test set document price
    let price_result = wasm_sdk::state_transitions::document::set_document_price(
        &contract_id,
        document_type,
        &test_document_id(),
        &owner_id,
        1000, // price
        1,    // revision
        1,    // identity nonce
        0,    // signature key id
    );
    assert!(price_result.is_ok(), "Should create set price transition");

    // Test purchase document
    let purchase_result = wasm_sdk::state_transitions::document::purchase_document(
        &contract_id,
        document_type,
        &test_document_id(),
        &test_identity_id(), // buyer
        &owner_id,           // seller
        1000,                // price
        1,                   // identity nonce
        0,                   // signature key id
    );
    assert!(
        purchase_result.is_ok(),
        "Should create purchase document transition"
    );
}

#[wasm_bindgen_test]
async fn test_complex_document_query() {
    let contract_id = test_contract_id();
    let document_type = "post";

    let query = DocumentQuery::new(&contract_id, document_type);
    assert!(query.is_ok());

    let mut q = query.unwrap();

    // Add multiple where clauses
    q.add_where_clause("author", "=", &test_identity_id().into());
    q.add_where_clause("likes", ">", &100.into());
    q.add_where_clause("tags", "contains", &"blockchain".into());
    q.add_where_clause("createdAt", ">=", &1234567890.into());

    // Add multiple order by clauses
    q.add_order_by("likes", false); // descending
    q.add_order_by("createdAt", false); // descending

    // Set pagination
    q.set_limit(20);
    q.set_offset(40);

    // Verify complex query
    let where_clauses = q.get_where_clauses().unwrap();
    assert_eq!(where_clauses.length(), 4, "Should have 4 where clauses");

    let order_by_clauses = q.get_order_by_clauses().unwrap();
    assert_eq!(
        order_by_clauses.length(),
        2,
        "Should have 2 order by clauses"
    );
}
