use super::create::DocumentCreateTransitionBuilder;
use super::delete::DocumentDeleteTransitionBuilder;
use super::purchase::DocumentPurchaseTransitionBuilder;
use super::replace::DocumentReplaceTransitionBuilder;
use super::set_price::DocumentSetPriceTransitionBuilder;
use super::transfer::DocumentTransferTransitionBuilder;
use crate::platform::test_helpers::{
    new_mock_sdk_with_contract_nonce, test_data_contract, test_identity_public_key,
    validate_transition_like_builder, TestSigner, INVALID_NONCE, TEST_DOCUMENT_TYPE_NAME,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use std::sync::Arc;

fn test_document(owner_id: Identifier) -> Document {
    Document::V0(DocumentV0 {
        id: Identifier::random(),
        owner_id,
        properties: Default::default(),
        revision: Some(1),
        created_at: None,
        updated_at: None,
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
        creator_id: None,
    })
}

pub(super) fn assert_document_create_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type,
        [7; 32],
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_delete_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_deletion_transition_from_document(
        document,
        document_type,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_purchase_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let purchaser_id = Identifier::random();
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_purchase_transition_from_document(
        document,
        document_type,
        purchaser_id,
        100,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_replace_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_replacement_transition_from_document(
        document,
        document_type,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_set_price_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_update_price_transition_from_document(
        document,
        document_type,
        200,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_document_transfer_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let recipient_id = Identifier::random();
    let document_type = data_contract
        .document_type_for_name(TEST_DOCUMENT_TYPE_NAME)
        .expect("expected test document type");
    let transition = BatchTransition::new_document_transfer_transition_from_document(
        document,
        document_type,
        recipient_id,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        None,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

#[tokio::test]
async fn document_builder_sign_masks_nonce_so_out_of_bounds_is_unreachable() {
    // Document builders obtain nonce through `Sdk::get_identity_contract_nonce`,
    // which masks out-of-bounds bits. This makes `validate_base_structure`
    // nonce-out-of-bounds errors unreachable through the builder API.
    // One test suffices since all document builders use the same SDK nonce path.
    let document_type_name = "testDoc";
    let data_contract = test_data_contract(document_type_name);
    let owner_id = Identifier::random();

    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 1_u64 << 50).await;

    let builder = DocumentDeleteTransitionBuilder::new(
        Arc::clone(&data_contract),
        document_type_name.to_string(),
        Identifier::random(),
        owner_id,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "SDK should mask nonce internally; got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn document_delete_builder_sign_succeeds_for_valid_input() {
    let document_type_name = "testDoc";
    let data_contract = test_data_contract(document_type_name);
    let owner_id = Identifier::random();
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let builder = DocumentDeleteTransitionBuilder::new(
        Arc::clone(&data_contract),
        document_type_name.to_string(),
        Identifier::random(),
        owner_id,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "valid input should pass validation; got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn document_create_builder_sign_succeeds_for_valid_input() {
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let builder = DocumentCreateTransitionBuilder::new(
        Arc::clone(&data_contract),
        TEST_DOCUMENT_TYPE_NAME.to_string(),
        document,
        [7; 32],
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "valid input should pass validation; got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn document_replace_builder_sign_succeeds_for_valid_input() {
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let builder = DocumentReplaceTransitionBuilder::new(
        Arc::clone(&data_contract),
        TEST_DOCUMENT_TYPE_NAME.to_string(),
        document,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "valid input should pass validation; got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn document_purchase_builder_sign_succeeds_for_valid_input() {
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let purchaser_id = Identifier::random();
    let sdk = new_mock_sdk_with_contract_nonce(purchaser_id, data_contract.id(), 0).await;

    let builder = DocumentPurchaseTransitionBuilder::new(
        Arc::clone(&data_contract),
        TEST_DOCUMENT_TYPE_NAME.to_string(),
        document,
        purchaser_id,
        100,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "valid input should pass validation; got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn document_set_price_builder_sign_succeeds_for_valid_input() {
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let builder = DocumentSetPriceTransitionBuilder::new(
        Arc::clone(&data_contract),
        TEST_DOCUMENT_TYPE_NAME.to_string(),
        document,
        200,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "valid input should pass validation; got error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn document_transfer_builder_sign_succeeds_for_valid_input() {
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let document = test_document(owner_id);
    let recipient_id = Identifier::random();
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let builder = DocumentTransferTransitionBuilder::new(
        Arc::clone(&data_contract),
        TEST_DOCUMENT_TYPE_NAME.to_string(),
        document,
        recipient_id,
    );

    let result = builder
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        result.is_ok(),
        "valid input should pass validation; got error: {:?}",
        result.err()
    );
}
