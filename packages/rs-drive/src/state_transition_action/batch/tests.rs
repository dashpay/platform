//! Comprehensive unit tests for batch action types.
//!
//! Tests cover every accessor method on both the V0 structs and
//! the enum wrappers for document actions, token actions,
//! `BatchedTransitionAction`, and `BatchTransitionAction`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;

use crate::drive::contract::DataContractFetchInfo;

// --- Document action imports ---
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
    DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0,
    DocumentBaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
    DocumentCreateTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::{
    DocumentDeleteTransitionActionAccessorsV0, DocumentDeleteTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{
    DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0,
    DocumentReplaceTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::{
    DocumentTransferTransitionAction, DocumentTransferTransitionActionAccessorsV0,
    DocumentTransferTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::{
    DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionAccessorsV0,
    DocumentPurchaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::{
    DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionAccessorsV0,
    DocumentUpdatePriceTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

// --- Token action imports ---
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
    TokenBaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::{
    TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0,
    TokenBurnTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{
    TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0,
    TokenMintTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::{
    TokenTransferTransitionAction, TokenTransferTransitionActionAccessorsV0,
    TokenTransferTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{
    TokenFreezeTransitionAction, TokenFreezeTransitionActionAccessorsV0,
    TokenFreezeTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::{
    TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionAccessorsV0,
    TokenUnfreezeTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::{
    TokenClaimTransitionAction, TokenClaimTransitionActionAccessorsV0,
    TokenClaimTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::{
    TokenEmergencyActionTransitionAction, TokenEmergencyActionTransitionActionAccessorsV0,
    TokenEmergencyActionTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{
    TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0,
    TokenDestroyFrozenFundsTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{
    TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0,
    TokenConfigUpdateTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::{
    TokenDirectPurchaseTransitionAction, TokenDirectPurchaseTransitionActionAccessorsV0,
    TokenDirectPurchaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::{
    TokenSetPriceForDirectPurchaseTransitionAction,
    TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0,
    TokenSetPriceForDirectPurchaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;

// --- Batch-level imports ---
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::v0::BatchTransitionActionV0;
use crate::state_transition_action::batch::BatchTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
    BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0,
};

// ============================================================
// Helper functions
// ============================================================

fn test_dpns_contract_info() -> Arc<DataContractFetchInfo> {
    Arc::new(DataContractFetchInfo::dpns_contract_fixture(
        PlatformVersion::latest().protocol_version,
    ))
}

fn test_document_base_v0() -> DocumentBaseTransitionActionV0 {
    DocumentBaseTransitionActionV0 {
        id: Identifier::from([0xAA; 32]),
        identity_contract_nonce: 1,
        document_type_name: "domain".to_string(),
        data_contract: test_dpns_contract_info(),
        token_cost: None,
        gas_fees_paid_by: GasFeesPaidBy::default(),
    }
}

fn test_document_base() -> DocumentBaseTransitionAction {
    DocumentBaseTransitionAction::V0(test_document_base_v0())
}

fn test_token_base_v0() -> TokenBaseTransitionActionV0 {
    TokenBaseTransitionActionV0 {
        token_id: Identifier::from([0xBB; 32]),
        identity_contract_nonce: 1,
        token_contract_position: 0,
        data_contract: test_dpns_contract_info(),
        store_in_group: None,
        perform_action: true,
    }
}

fn test_token_base() -> TokenBaseTransitionAction {
    TokenBaseTransitionAction::V0(test_token_base_v0())
}

fn test_document() -> Document {
    Document::V0(DocumentV0::default())
}

// ============================================================
// 1. DocumentBaseTransitionAction tests
// ============================================================

#[test]
fn test_document_base_v0_id() {
    let base = test_document_base_v0();
    assert_eq!(base.id, Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_base_action_id() {
    let base = test_document_base();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_base_action_document_type_name() {
    let base = test_document_base();
    assert_eq!(base.document_type_name(), "domain");
}

#[test]
fn test_document_base_action_document_type_name_owned() {
    let base = test_document_base();
    assert_eq!(base.document_type_name_owned(), "domain".to_string());
}

#[test]
fn test_document_base_action_data_contract_id() {
    let base = test_document_base();
    let contract_id = base.data_contract_id();
    // Verify the contract_id matches the one from the same base's fetch info
    assert_eq!(contract_id, base.data_contract_fetch_info().contract.id());
}

#[test]
fn test_document_base_action_data_contract_fetch_info() {
    let base = test_document_base();
    let info = base.data_contract_fetch_info();
    // Verify we get a non-default identifier (DPNS contract has a real id)
    assert_ne!(info.contract.id(), Identifier::default());
}

#[test]
fn test_document_base_action_data_contract_fetch_info_ref() {
    let base = test_document_base();
    let info_ref = base.data_contract_fetch_info_ref();
    // The ref and the cloned Arc should point to the same contract
    assert_eq!(info_ref.contract.id(), base.data_contract_fetch_info().contract.id());
}

#[test]
fn test_document_base_action_identity_contract_nonce() {
    let base = test_document_base();
    assert_eq!(base.identity_contract_nonce(), 1);
}

#[test]
fn test_document_base_action_token_cost_none() {
    let base = test_document_base();
    assert!(base.token_cost().is_none());
}

#[test]
fn test_document_base_action_document_type() {
    let base = test_document_base();
    let doc_type = base.document_type();
    assert!(doc_type.is_ok());
}

#[test]
fn test_document_base_action_document_type_field_is_required() {
    let base = test_document_base();
    // "label" is a required field on the DPNS domain document type
    let result = base.document_type_field_is_required("label");
    assert!(result.is_ok());
}

// ============================================================
// 2. DocumentCreateTransitionAction tests
// ============================================================

fn make_create_v0() -> DocumentCreateTransitionActionV0 {
    DocumentCreateTransitionActionV0 {
        base: test_document_base(),
        block_info: BlockInfo::default(),
        data: BTreeMap::from([("key".to_string(), Value::Text("value".to_string()))]),
        prefunded_voting_balance: None,
        current_store_contest_info: None,
        should_store_contest_info: None,
    }
}

fn make_create() -> DocumentCreateTransitionAction {
    DocumentCreateTransitionAction::V0(make_create_v0())
}

#[test]
fn test_create_v0_base() {
    let action = make_create_v0();
    assert_eq!(action.base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_create_action_base() {
    let action = make_create();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_create_action_base_owned() {
    let action = make_create();
    let base = action.base_owned();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_create_action_block_info() {
    let action = make_create();
    let bi = action.block_info();
    assert_eq!(bi, BlockInfo::default());
}

#[test]
fn test_create_action_data() {
    let action = make_create();
    assert_eq!(
        action.data().get("key"),
        Some(&Value::Text("value".to_string()))
    );
}

#[test]
fn test_create_action_data_mut() {
    let mut action = make_create();
    action
        .data_mut()
        .insert("new_key".to_string(), Value::Bool(true));
    assert_eq!(action.data().get("new_key"), Some(&Value::Bool(true)));
}

#[test]
fn test_create_action_data_owned() {
    let action = make_create();
    let data = action.data_owned();
    assert_eq!(data.get("key"), Some(&Value::Text("value".to_string())));
}

#[test]
fn test_create_action_prefunded_voting_balance_none() {
    let action = make_create();
    assert!(action.prefunded_voting_balance().is_none());
}

#[test]
fn test_create_action_take_prefunded_voting_balance_none() {
    let mut action = make_create();
    assert!(action.take_prefunded_voting_balance().is_none());
}

#[test]
fn test_create_action_should_store_contest_info_none() {
    let action = make_create();
    assert!(action.should_store_contest_info().is_none());
}

#[test]
fn test_create_action_take_should_store_contest_info_none() {
    let mut action = make_create();
    assert!(action.take_should_store_contest_info().is_none());
}

#[test]
fn test_create_action_current_store_contest_info_none() {
    let action = make_create();
    assert!(action.current_store_contest_info().is_none());
}

#[test]
fn test_create_action_take_current_store_contest_info_none() {
    let mut action = make_create();
    assert!(action.take_current_store_contest_info().is_none());
}

// ============================================================
// 3. DocumentDeleteTransitionAction tests
// ============================================================

fn make_delete_v0() -> DocumentDeleteTransitionActionV0 {
    DocumentDeleteTransitionActionV0 {
        base: test_document_base(),
    }
}

fn make_delete() -> DocumentDeleteTransitionAction {
    DocumentDeleteTransitionAction::V0(make_delete_v0())
}

#[test]
fn test_delete_v0_base() {
    let action = make_delete_v0();
    assert_eq!(action.base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_delete_action_base() {
    let action = make_delete();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_delete_action_base_owned() {
    let action = make_delete();
    let base = action.base_owned();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

// ============================================================
// 4. DocumentReplaceTransitionAction tests
// ============================================================

fn make_replace_v0() -> DocumentReplaceTransitionActionV0 {
    DocumentReplaceTransitionActionV0 {
        base: test_document_base(),
        revision: 2,
        created_at: Some(1000),
        updated_at: Some(2000),
        transferred_at: Some(3000),
        created_at_block_height: Some(10),
        updated_at_block_height: Some(20),
        transferred_at_block_height: Some(30),
        created_at_core_block_height: Some(100),
        updated_at_core_block_height: Some(200),
        transferred_at_core_block_height: Some(300),
        data: BTreeMap::from([("field".to_string(), Value::U64(42))]),
        changed_data_fields: BTreeSet::from(["field".to_string()]),
        creator_id: Some(Identifier::from([0xCC; 32])),
    }
}

fn make_replace() -> DocumentReplaceTransitionAction {
    DocumentReplaceTransitionAction::V0(make_replace_v0())
}

#[test]
fn test_replace_action_base() {
    let action = make_replace();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_replace_action_base_owned() {
    let action = make_replace();
    let base = action.base_owned();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_replace_action_revision() {
    let action = make_replace();
    assert_eq!(action.revision(), 2);
}

#[test]
fn test_replace_action_created_at() {
    let action = make_replace();
    assert_eq!(action.created_at(), Some(1000));
}

#[test]
fn test_replace_action_updated_at() {
    let action = make_replace();
    assert_eq!(action.updated_at(), Some(2000));
}

#[test]
fn test_replace_action_transferred_at() {
    let action = make_replace();
    assert_eq!(action.transferred_at(), Some(3000));
}

#[test]
fn test_replace_action_created_at_block_height() {
    let action = make_replace();
    assert_eq!(action.created_at_block_height(), Some(10));
}

#[test]
fn test_replace_action_updated_at_block_height() {
    let action = make_replace();
    assert_eq!(action.updated_at_block_height(), Some(20));
}

#[test]
fn test_replace_action_transferred_at_block_height() {
    let action = make_replace();
    assert_eq!(action.transferred_at_block_height(), Some(30));
}

#[test]
fn test_replace_action_created_at_core_block_height() {
    let action = make_replace();
    assert_eq!(action.created_at_core_block_height(), Some(100));
}

#[test]
fn test_replace_action_updated_at_core_block_height() {
    let action = make_replace();
    assert_eq!(action.updated_at_core_block_height(), Some(200));
}

#[test]
fn test_replace_action_transferred_at_core_block_height() {
    let action = make_replace();
    assert_eq!(action.transferred_at_core_block_height(), Some(300));
}

#[test]
fn test_replace_action_data() {
    let action = make_replace();
    assert_eq!(action.data().get("field"), Some(&Value::U64(42)));
}

#[test]
fn test_replace_action_changed_data_fields() {
    let action = make_replace();
    assert!(action.changed_data_fields().contains("field"));
}

#[test]
fn test_replace_action_data_owned() {
    let action = make_replace();
    let data = action.data_owned();
    assert_eq!(data.get("field"), Some(&Value::U64(42)));
}

#[test]
fn test_replace_action_creator_id() {
    let action = make_replace();
    assert_eq!(action.creator_id(), Some(Identifier::from([0xCC; 32])));
}

// ============================================================
// 5. DocumentTransferTransitionAction tests
// ============================================================

fn make_transfer_v0() -> DocumentTransferTransitionActionV0 {
    DocumentTransferTransitionActionV0 {
        base: test_document_base(),
        document: test_document(),
    }
}

fn make_transfer() -> DocumentTransferTransitionAction {
    DocumentTransferTransitionAction::V0(make_transfer_v0())
}

#[test]
fn test_transfer_action_base() {
    let action = make_transfer();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_transfer_action_base_owned() {
    let action = make_transfer();
    let base = action.base_owned();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_transfer_action_document() {
    let action = make_transfer();
    let _doc = action.document(); // just verify it doesn't panic
}

#[test]
fn test_transfer_action_document_owned() {
    let action = make_transfer();
    let _doc = action.document_owned();
}

// ============================================================
// 6. DocumentPurchaseTransitionAction tests
// ============================================================

fn make_purchase_v0() -> DocumentPurchaseTransitionActionV0 {
    DocumentPurchaseTransitionActionV0 {
        base: test_document_base(),
        document: test_document(),
        original_owner_id: Identifier::from([0xDD; 32]),
        price: 5000,
    }
}

fn make_purchase() -> DocumentPurchaseTransitionAction {
    DocumentPurchaseTransitionAction::V0(make_purchase_v0())
}

#[test]
fn test_purchase_action_base() {
    let action = make_purchase();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_purchase_action_base_owned() {
    let action = make_purchase();
    let base = action.base_owned();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_purchase_action_document() {
    let action = make_purchase();
    let _doc = action.document();
}

#[test]
fn test_purchase_action_document_owned() {
    let action = make_purchase();
    let _doc = action.document_owned();
}

#[test]
fn test_purchase_action_original_owner_id() {
    let action = make_purchase();
    assert_eq!(action.original_owner_id(), Identifier::from([0xDD; 32]));
}

#[test]
fn test_purchase_action_price() {
    let action = make_purchase();
    assert_eq!(action.price(), 5000);
}

// ============================================================
// 7. DocumentUpdatePriceTransitionAction tests
// ============================================================

fn make_update_price_v0() -> DocumentUpdatePriceTransitionActionV0 {
    DocumentUpdatePriceTransitionActionV0 {
        base: test_document_base(),
        document: test_document(),
    }
}

fn make_update_price() -> DocumentUpdatePriceTransitionAction {
    DocumentUpdatePriceTransitionAction::V0(make_update_price_v0())
}

#[test]
fn test_update_price_action_base() {
    let action = make_update_price();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_update_price_action_base_owned() {
    let action = make_update_price();
    let base = action.base_owned();
    assert_eq!(base.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_update_price_action_document() {
    let action = make_update_price();
    let _doc = action.document();
}

#[test]
fn test_update_price_action_document_owned() {
    let action = make_update_price();
    let _doc = action.document_owned();
}

// ============================================================
// 8. DocumentTransitionAction enum tests
// ============================================================

#[test]
fn test_document_transition_action_create_base() {
    let action: DocumentTransitionAction = make_create().into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_create_base_owned() {
    let action: DocumentTransitionAction = make_create().into();
    assert_eq!(action.base_owned().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_delete_base() {
    let action: DocumentTransitionAction = make_delete().into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_delete_base_owned() {
    let action: DocumentTransitionAction = make_delete().into();
    assert_eq!(action.base_owned().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_replace_base() {
    let action: DocumentTransitionAction = make_replace().into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_replace_base_owned() {
    let action: DocumentTransitionAction = make_replace().into();
    assert_eq!(action.base_owned().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_transfer_base() {
    let action: DocumentTransitionAction = make_transfer().into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_transfer_base_owned() {
    let action: DocumentTransitionAction = make_transfer().into();
    assert_eq!(action.base_owned().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_purchase_base() {
    let action: DocumentTransitionAction = make_purchase().into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_purchase_base_owned() {
    let action: DocumentTransitionAction = make_purchase().into();
    assert_eq!(action.base_owned().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_update_price_base() {
    let action: DocumentTransitionAction = make_update_price().into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_transition_action_update_price_base_owned() {
    let action: DocumentTransitionAction = make_update_price().into();
    assert_eq!(action.base_owned().id(), Identifier::from([0xAA; 32]));
}

// ============================================================
// 9. TokenBaseTransitionAction tests
// ============================================================

#[test]
fn test_token_base_v0_token_position() {
    let base = test_token_base_v0();
    assert_eq!(base.token_position(), 0);
}

#[test]
fn test_token_base_v0_token_id() {
    let base = test_token_base_v0();
    assert_eq!(base.token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_base_v0_data_contract_id() {
    let base = test_token_base_v0();
    let contract_id = base.data_contract_id();
    // Verify the contract_id matches the one from the same base's fetch info
    assert_eq!(contract_id, base.data_contract_fetch_info().contract.id());
}

#[test]
fn test_token_base_v0_data_contract_fetch_info_ref() {
    let base = test_token_base_v0();
    let info = base.data_contract_fetch_info_ref();
    assert_ne!(info.contract.id(), Identifier::default());
}

#[test]
fn test_token_base_v0_data_contract_fetch_info() {
    let base = test_token_base_v0();
    let info = base.data_contract_fetch_info();
    assert_ne!(info.contract.id(), Identifier::default());
}

#[test]
fn test_token_base_v0_identity_contract_nonce() {
    let base = test_token_base_v0();
    assert_eq!(base.identity_contract_nonce(), 1);
}

#[test]
fn test_token_base_v0_store_in_group_none() {
    let base = test_token_base_v0();
    assert!(base.store_in_group().is_none());
}

#[test]
fn test_token_base_v0_original_group_action_none() {
    let base = test_token_base_v0();
    assert!(base.original_group_action().is_none());
}

#[test]
fn test_token_base_v0_perform_action() {
    let base = test_token_base_v0();
    assert!(base.perform_action());
}

// token_configuration() will fail for DPNS since it has no tokens at position 0
#[test]
fn test_token_base_v0_token_configuration_fails_for_dpns() {
    let base = test_token_base_v0();
    assert!(base.token_configuration().is_err());
}

// Now test the enum wrapper
#[test]
fn test_token_base_action_token_position() {
    let base = test_token_base();
    assert_eq!(base.token_position(), 0);
}

#[test]
fn test_token_base_action_token_id() {
    let base = test_token_base();
    assert_eq!(base.token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_base_action_data_contract_id() {
    let base = test_token_base();
    let contract_id = base.data_contract_id();
    assert_eq!(contract_id, base.data_contract_fetch_info().contract.id());
}

#[test]
fn test_token_base_action_data_contract_fetch_info() {
    let base = test_token_base();
    let info = base.data_contract_fetch_info();
    assert_ne!(info.contract.id(), Identifier::default());
}

#[test]
fn test_token_base_action_data_contract_fetch_info_ref() {
    let base = test_token_base();
    let info = base.data_contract_fetch_info_ref();
    assert_eq!(
        info.contract.id(),
        base.data_contract_fetch_info().contract.id()
    );
}

#[test]
fn test_token_base_action_identity_contract_nonce() {
    let base = test_token_base();
    assert_eq!(base.identity_contract_nonce(), 1);
}

#[test]
fn test_token_base_action_store_in_group_none() {
    let base = test_token_base();
    assert!(base.store_in_group().is_none());
}

#[test]
fn test_token_base_action_original_group_action_none() {
    let base = test_token_base();
    assert!(base.original_group_action().is_none());
}

#[test]
fn test_token_base_action_perform_action() {
    let base = test_token_base();
    assert!(base.perform_action());
}

#[test]
fn test_token_base_action_token_configuration_fails_for_dpns() {
    let base = test_token_base();
    assert!(base.token_configuration().is_err());
}

// ============================================================
// 10. TokenBurnTransitionAction tests
// ============================================================

fn make_burn_v0() -> TokenBurnTransitionActionV0 {
    TokenBurnTransitionActionV0 {
        base: test_token_base(),
        burn_from_identifier: Identifier::from([0xCC; 32]),
        burn_amount: 500,
        public_note: Some("burn note".to_string()),
    }
}

fn make_burn() -> TokenBurnTransitionAction {
    TokenBurnTransitionAction::V0(make_burn_v0())
}

#[test]
fn test_burn_v0_base() {
    let action = make_burn_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_burn_v0_burn_from_identifier() {
    let action = make_burn_v0();
    assert_eq!(action.burn_from_identifier(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_burn_v0_burn_amount() {
    let action = make_burn_v0();
    assert_eq!(action.burn_amount(), 500);
}

#[test]
fn test_burn_v0_public_note() {
    let action = make_burn_v0();
    assert_eq!(action.public_note(), Some(&"burn note".to_string()));
}

#[test]
fn test_burn_v0_public_note_owned() {
    let action = make_burn_v0();
    assert_eq!(action.public_note_owned(), Some("burn note".to_string()));
}

#[test]
fn test_burn_v0_set_burn_from_identifier() {
    let mut action = make_burn_v0();
    let new_id = Identifier::from([0xEE; 32]);
    action.set_burn_from_identifier(new_id);
    assert_eq!(action.burn_from_identifier(), new_id);
}

#[test]
fn test_burn_v0_set_burn_amount() {
    let mut action = make_burn_v0();
    action.set_burn_amount(999);
    assert_eq!(action.burn_amount(), 999);
}

#[test]
fn test_burn_v0_set_public_note() {
    let mut action = make_burn_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper tests
#[test]
fn test_burn_action_base() {
    let action = make_burn();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_burn_action_base_owned() {
    let action = make_burn();
    assert_eq!(action.base_owned().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_burn_action_burn_from_identifier() {
    let action = make_burn();
    assert_eq!(action.burn_from_identifier(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_burn_action_set_burn_from_identifier() {
    let mut action = make_burn();
    let new_id = Identifier::from([0xEE; 32]);
    action.set_burn_from_identifier(new_id);
    assert_eq!(action.burn_from_identifier(), new_id);
}

#[test]
fn test_burn_action_burn_amount() {
    let action = make_burn();
    assert_eq!(action.burn_amount(), 500);
}

#[test]
fn test_burn_action_set_burn_amount() {
    let mut action = make_burn();
    action.set_burn_amount(999);
    assert_eq!(action.burn_amount(), 999);
}

#[test]
fn test_burn_action_public_note() {
    let action = make_burn();
    assert_eq!(action.public_note(), Some(&"burn note".to_string()));
}

#[test]
fn test_burn_action_public_note_owned() {
    let action = make_burn();
    assert_eq!(action.public_note_owned(), Some("burn note".to_string()));
}

#[test]
fn test_burn_action_set_public_note() {
    let mut action = make_burn();
    action.set_public_note(Some("new note".to_string()));
    assert_eq!(action.public_note(), Some(&"new note".to_string()));
}

// ============================================================
// 11. TokenMintTransitionAction tests
// ============================================================

fn make_mint_v0() -> TokenMintTransitionActionV0 {
    TokenMintTransitionActionV0 {
        base: test_token_base(),
        mint_amount: 1000,
        identity_balance_holder_id: Identifier::from([0xDD; 32]),
        public_note: Some("mint note".to_string()),
    }
}

fn make_mint() -> TokenMintTransitionAction {
    TokenMintTransitionAction::V0(make_mint_v0())
}

#[test]
fn test_mint_v0_base() {
    let action = make_mint_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_mint_v0_mint_amount() {
    let action = make_mint_v0();
    assert_eq!(action.mint_amount(), 1000);
}

#[test]
fn test_mint_v0_identity_balance_holder_id() {
    let action = make_mint_v0();
    assert_eq!(
        action.identity_balance_holder_id(),
        Identifier::from([0xDD; 32])
    );
}

#[test]
fn test_mint_v0_public_note() {
    let action = make_mint_v0();
    assert_eq!(action.public_note(), Some(&"mint note".to_string()));
}

#[test]
fn test_mint_v0_set_mint_amount() {
    let mut action = make_mint_v0();
    action.set_mint_amount(2000);
    assert_eq!(action.mint_amount(), 2000);
}

#[test]
fn test_mint_v0_set_identity_balance_holder_id() {
    let mut action = make_mint_v0();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_identity_balance_holder_id(new_id);
    assert_eq!(action.identity_balance_holder_id(), new_id);
}

#[test]
fn test_mint_v0_set_public_note() {
    let mut action = make_mint_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

#[test]
fn test_mint_v0_public_note_owned() {
    let action = make_mint_v0();
    assert_eq!(action.public_note_owned(), Some("mint note".to_string()));
}

// Enum wrapper
#[test]
fn test_mint_action_base() {
    let action = make_mint();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_mint_action_base_owned() {
    let action = make_mint();
    assert_eq!(action.base_owned().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_mint_action_mint_amount() {
    let action = make_mint();
    assert_eq!(action.mint_amount(), 1000);
}

#[test]
fn test_mint_action_set_mint_amount() {
    let mut action = make_mint();
    action.set_mint_amount(2000);
    assert_eq!(action.mint_amount(), 2000);
}

#[test]
fn test_mint_action_identity_balance_holder_id() {
    let action = make_mint();
    assert_eq!(
        action.identity_balance_holder_id(),
        Identifier::from([0xDD; 32])
    );
}

#[test]
fn test_mint_action_set_identity_balance_holder_id() {
    let mut action = make_mint();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_identity_balance_holder_id(new_id);
    assert_eq!(action.identity_balance_holder_id(), new_id);
}

#[test]
fn test_mint_action_public_note() {
    let action = make_mint();
    assert_eq!(action.public_note(), Some(&"mint note".to_string()));
}

#[test]
fn test_mint_action_public_note_owned() {
    let action = make_mint();
    assert_eq!(action.public_note_owned(), Some("mint note".to_string()));
}

#[test]
fn test_mint_action_set_public_note() {
    let mut action = make_mint();
    action.set_public_note(Some("changed".to_string()));
    assert_eq!(action.public_note(), Some(&"changed".to_string()));
}

// ============================================================
// 12. TokenTransferTransitionAction tests
// ============================================================

fn make_token_transfer_v0() -> TokenTransferTransitionActionV0 {
    TokenTransferTransitionActionV0 {
        base: test_token_base(),
        amount: 750,
        recipient_id: Identifier::from([0xEE; 32]),
        public_note: Some("transfer note".to_string()),
        shared_encrypted_note: None,
        private_encrypted_note: None,
    }
}

fn make_token_transfer() -> TokenTransferTransitionAction {
    TokenTransferTransitionAction::V0(make_token_transfer_v0())
}

#[test]
fn test_token_transfer_v0_base() {
    let action = make_token_transfer_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transfer_v0_amount() {
    let action = make_token_transfer_v0();
    assert_eq!(action.amount(), 750);
}

#[test]
fn test_token_transfer_v0_recipient_id() {
    let action = make_token_transfer_v0();
    assert_eq!(action.recipient_id(), Identifier::from([0xEE; 32]));
}

#[test]
fn test_token_transfer_v0_public_note() {
    let action = make_token_transfer_v0();
    assert_eq!(action.public_note(), Some(&"transfer note".to_string()));
}

#[test]
fn test_token_transfer_v0_public_note_owned() {
    let action = make_token_transfer_v0();
    assert_eq!(
        action.public_note_owned(),
        Some("transfer note".to_string())
    );
}

#[test]
fn test_token_transfer_v0_set_public_note() {
    let mut action = make_token_transfer_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

#[test]
fn test_token_transfer_v0_shared_encrypted_note_none() {
    let action = make_token_transfer_v0();
    assert!(action.shared_encrypted_note().is_none());
}

#[test]
fn test_token_transfer_v0_shared_encrypted_note_owned_none() {
    let action = make_token_transfer_v0();
    assert!(action.shared_encrypted_note_owned().is_none());
}

#[test]
fn test_token_transfer_v0_set_shared_encrypted_note() {
    let mut action = make_token_transfer_v0();
    let note = (0u32, 1u32, vec![1, 2, 3]);
    action.set_shared_encrypted_note(Some(note.clone()));
    assert_eq!(action.shared_encrypted_note(), Some(&note));
}

#[test]
fn test_token_transfer_v0_private_encrypted_note_none() {
    let action = make_token_transfer_v0();
    assert!(action.private_encrypted_note().is_none());
}

#[test]
fn test_token_transfer_v0_private_encrypted_note_owned_none() {
    let action = make_token_transfer_v0();
    assert!(action.private_encrypted_note_owned().is_none());
}

#[test]
fn test_token_transfer_v0_set_private_encrypted_note() {
    let mut action = make_token_transfer_v0();
    let note = (0u32, 1u32, vec![4, 5, 6]);
    action.set_private_encrypted_note(Some(note.clone()));
    assert_eq!(action.private_encrypted_note(), Some(&note));
}

#[test]
fn test_token_transfer_v0_notes() {
    let action = make_token_transfer_v0();
    let (public, shared, private) = action.notes();
    assert_eq!(public, Some("transfer note".to_string()));
    assert!(shared.is_none());
    assert!(private.is_none());
}

#[test]
fn test_token_transfer_v0_notes_owned() {
    let action = make_token_transfer_v0();
    let (public, shared, private) = action.notes_owned();
    assert_eq!(public, Some("transfer note".to_string()));
    assert!(shared.is_none());
    assert!(private.is_none());
}

// Enum wrapper
#[test]
fn test_token_transfer_action_base() {
    let action = make_token_transfer();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transfer_action_base_owned() {
    let action = make_token_transfer();
    assert_eq!(action.base_owned().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transfer_action_amount() {
    let action = make_token_transfer();
    assert_eq!(action.amount(), 750);
}

#[test]
fn test_token_transfer_action_recipient_id() {
    let action = make_token_transfer();
    assert_eq!(action.recipient_id(), Identifier::from([0xEE; 32]));
}

#[test]
fn test_token_transfer_action_public_note() {
    let action = make_token_transfer();
    assert_eq!(action.public_note(), Some(&"transfer note".to_string()));
}

#[test]
fn test_token_transfer_action_public_note_owned() {
    let action = make_token_transfer();
    assert_eq!(
        action.public_note_owned(),
        Some("transfer note".to_string())
    );
}

#[test]
fn test_token_transfer_action_set_public_note() {
    let mut action = make_token_transfer();
    action.set_public_note(Some("x".to_string()));
    assert_eq!(action.public_note(), Some(&"x".to_string()));
}

#[test]
fn test_token_transfer_action_shared_encrypted_note() {
    let action = make_token_transfer();
    assert!(action.shared_encrypted_note().is_none());
}

#[test]
fn test_token_transfer_action_shared_encrypted_note_owned() {
    let action = make_token_transfer();
    assert!(action.shared_encrypted_note_owned().is_none());
}

#[test]
fn test_token_transfer_action_set_shared_encrypted_note() {
    let mut action = make_token_transfer();
    let note = (0u32, 1u32, vec![7, 8]);
    action.set_shared_encrypted_note(Some(note));
    assert!(action.shared_encrypted_note().is_some());
}

#[test]
fn test_token_transfer_action_private_encrypted_note() {
    let action = make_token_transfer();
    assert!(action.private_encrypted_note().is_none());
}

#[test]
fn test_token_transfer_action_private_encrypted_note_owned() {
    let action = make_token_transfer();
    assert!(action.private_encrypted_note_owned().is_none());
}

#[test]
fn test_token_transfer_action_set_private_encrypted_note() {
    let mut action = make_token_transfer();
    let note = (0u32, 1u32, vec![9, 10]);
    action.set_private_encrypted_note(Some(note));
    assert!(action.private_encrypted_note().is_some());
}

#[test]
fn test_token_transfer_action_notes() {
    let action = make_token_transfer();
    let (public, shared, private) = action.notes();
    assert!(public.is_some());
    assert!(shared.is_none());
    assert!(private.is_none());
}

#[test]
fn test_token_transfer_action_notes_owned() {
    let action = make_token_transfer();
    let (public, shared, private) = action.notes_owned();
    assert!(public.is_some());
    assert!(shared.is_none());
    assert!(private.is_none());
}

// ============================================================
// 13. TokenFreezeTransitionAction tests
// ============================================================

fn make_freeze_v0() -> TokenFreezeTransitionActionV0 {
    TokenFreezeTransitionActionV0 {
        base: test_token_base(),
        identity_to_freeze_id: Identifier::from([0xCC; 32]),
        public_note: Some("freeze note".to_string()),
    }
}

fn make_freeze() -> TokenFreezeTransitionAction {
    TokenFreezeTransitionAction::V0(make_freeze_v0())
}

#[test]
fn test_freeze_v0_base() {
    let action = make_freeze_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_freeze_v0_identity_to_freeze_id() {
    let action = make_freeze_v0();
    assert_eq!(
        action.identity_to_freeze_id(),
        Identifier::from([0xCC; 32])
    );
}

#[test]
fn test_freeze_v0_set_identity_to_freeze_id() {
    let mut action = make_freeze_v0();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_identity_to_freeze_id(new_id);
    assert_eq!(action.identity_to_freeze_id(), new_id);
}

#[test]
fn test_freeze_v0_public_note() {
    let action = make_freeze_v0();
    assert_eq!(action.public_note(), Some(&"freeze note".to_string()));
}

#[test]
fn test_freeze_v0_public_note_owned() {
    let action = make_freeze_v0();
    assert_eq!(action.public_note_owned(), Some("freeze note".to_string()));
}

#[test]
fn test_freeze_v0_set_public_note() {
    let mut action = make_freeze_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_freeze_action_base() {
    let action = make_freeze();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_freeze_action_base_owned() {
    let action = make_freeze();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_freeze_action_identity_to_freeze_id() {
    let action = make_freeze();
    assert_eq!(
        action.identity_to_freeze_id(),
        Identifier::from([0xCC; 32])
    );
}

#[test]
fn test_freeze_action_set_identity_to_freeze_id() {
    let mut action = make_freeze();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_identity_to_freeze_id(new_id);
    assert_eq!(action.identity_to_freeze_id(), new_id);
}

#[test]
fn test_freeze_action_public_note() {
    let action = make_freeze();
    assert_eq!(action.public_note(), Some(&"freeze note".to_string()));
}

#[test]
fn test_freeze_action_public_note_owned() {
    let action = make_freeze();
    assert_eq!(action.public_note_owned(), Some("freeze note".to_string()));
}

#[test]
fn test_freeze_action_set_public_note() {
    let mut action = make_freeze();
    action.set_public_note(Some("changed".to_string()));
    assert_eq!(action.public_note(), Some(&"changed".to_string()));
}

// ============================================================
// 14. TokenUnfreezeTransitionAction tests
// ============================================================

fn make_unfreeze_v0() -> TokenUnfreezeTransitionActionV0 {
    TokenUnfreezeTransitionActionV0 {
        base: test_token_base(),
        frozen_identity_id: Identifier::from([0xCC; 32]),
        public_note: Some("unfreeze note".to_string()),
    }
}

fn make_unfreeze() -> TokenUnfreezeTransitionAction {
    TokenUnfreezeTransitionAction::V0(make_unfreeze_v0())
}

#[test]
fn test_unfreeze_v0_base() {
    let action = make_unfreeze_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_unfreeze_v0_frozen_identity_id() {
    let action = make_unfreeze_v0();
    assert_eq!(action.frozen_identity_id(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_unfreeze_v0_set_frozen_identity_id() {
    let mut action = make_unfreeze_v0();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_frozen_identity_id(new_id);
    assert_eq!(action.frozen_identity_id(), new_id);
}

#[test]
fn test_unfreeze_v0_public_note() {
    let action = make_unfreeze_v0();
    assert_eq!(action.public_note(), Some(&"unfreeze note".to_string()));
}

#[test]
fn test_unfreeze_v0_public_note_owned() {
    let action = make_unfreeze_v0();
    assert_eq!(
        action.public_note_owned(),
        Some("unfreeze note".to_string())
    );
}

#[test]
fn test_unfreeze_v0_set_public_note() {
    let mut action = make_unfreeze_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_unfreeze_action_base() {
    let action = make_unfreeze();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_unfreeze_action_base_owned() {
    let action = make_unfreeze();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_unfreeze_action_frozen_identity_id() {
    let action = make_unfreeze();
    assert_eq!(action.frozen_identity_id(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_unfreeze_action_set_frozen_identity_id() {
    let mut action = make_unfreeze();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_frozen_identity_id(new_id);
    assert_eq!(action.frozen_identity_id(), new_id);
}

#[test]
fn test_unfreeze_action_public_note() {
    let action = make_unfreeze();
    assert_eq!(action.public_note(), Some(&"unfreeze note".to_string()));
}

#[test]
fn test_unfreeze_action_public_note_owned() {
    let action = make_unfreeze();
    assert_eq!(
        action.public_note_owned(),
        Some("unfreeze note".to_string())
    );
}

#[test]
fn test_unfreeze_action_set_public_note() {
    let mut action = make_unfreeze();
    action.set_public_note(Some("x".to_string()));
    assert_eq!(action.public_note(), Some(&"x".to_string()));
}

// ============================================================
// 15. TokenClaimTransitionAction tests
// ============================================================

fn make_claim_v0() -> TokenClaimTransitionActionV0 {
    TokenClaimTransitionActionV0 {
        base: test_token_base(),
        amount: 300,
        distribution_info: TokenDistributionInfo::PreProgrammed(
            1000,
            Identifier::from([0xDD; 32]),
        ),
        public_note: Some("claim note".to_string()),
    }
}

fn make_claim() -> TokenClaimTransitionAction {
    TokenClaimTransitionAction::V0(make_claim_v0())
}

#[test]
fn test_claim_v0_base() {
    let action = make_claim_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_claim_v0_amount() {
    let action = make_claim_v0();
    assert_eq!(action.amount(), 300);
}

#[test]
fn test_claim_v0_set_amount() {
    let mut action = make_claim_v0();
    action.set_amount(600);
    assert_eq!(action.amount(), 600);
}

#[test]
fn test_claim_v0_distribution_info() {
    let action = make_claim_v0();
    match action.distribution_info() {
        TokenDistributionInfo::PreProgrammed(ts, id) => {
            assert_eq!(*ts, 1000);
            assert_eq!(*id, Identifier::from([0xDD; 32]));
        }
        _ => panic!("Expected PreProgrammed variant"),
    }
}

#[test]
fn test_claim_v0_set_distribution_info() {
    let mut action = make_claim_v0();
    let new_info =
        TokenDistributionInfo::PreProgrammed(2000, Identifier::from([0xEE; 32]));
    action.set_distribution_info(new_info);
    match action.distribution_info() {
        TokenDistributionInfo::PreProgrammed(ts, id) => {
            assert_eq!(*ts, 2000);
            assert_eq!(*id, Identifier::from([0xEE; 32]));
        }
        _ => panic!("Expected PreProgrammed variant"),
    }
}

#[test]
fn test_claim_v0_public_note() {
    let action = make_claim_v0();
    assert_eq!(action.public_note(), Some(&"claim note".to_string()));
}

#[test]
fn test_claim_v0_public_note_owned() {
    let action = make_claim_v0();
    assert_eq!(action.public_note_owned(), Some("claim note".to_string()));
}

#[test]
fn test_claim_v0_set_public_note() {
    let mut action = make_claim_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_claim_action_base() {
    let action = make_claim();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_claim_action_base_owned() {
    let action = make_claim();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_claim_action_amount() {
    let action = make_claim();
    assert_eq!(action.amount(), 300);
}

#[test]
fn test_claim_action_set_amount() {
    let mut action = make_claim();
    action.set_amount(600);
    assert_eq!(action.amount(), 600);
}

#[test]
fn test_claim_action_distribution_info() {
    let action = make_claim();
    match action.distribution_info() {
        TokenDistributionInfo::PreProgrammed(ts, _) => assert_eq!(*ts, 1000),
        _ => panic!("Expected PreProgrammed"),
    }
}

#[test]
fn test_claim_action_set_distribution_info() {
    let mut action = make_claim();
    let new_info =
        TokenDistributionInfo::PreProgrammed(5000, Identifier::from([0xFF; 32]));
    action.set_distribution_info(new_info);
    match action.distribution_info() {
        TokenDistributionInfo::PreProgrammed(ts, id) => {
            assert_eq!(*ts, 5000);
            assert_eq!(*id, Identifier::from([0xFF; 32]));
        }
        _ => panic!("Expected PreProgrammed"),
    }
}

#[test]
fn test_claim_action_public_note() {
    let action = make_claim();
    assert_eq!(action.public_note(), Some(&"claim note".to_string()));
}

#[test]
fn test_claim_action_public_note_owned() {
    let action = make_claim();
    assert_eq!(action.public_note_owned(), Some("claim note".to_string()));
}

#[test]
fn test_claim_action_set_public_note() {
    let mut action = make_claim();
    action.set_public_note(Some("new".to_string()));
    assert_eq!(action.public_note(), Some(&"new".to_string()));
}

// ============================================================
// 16. TokenEmergencyActionTransitionAction tests
// ============================================================

fn make_emergency_v0() -> TokenEmergencyActionTransitionActionV0 {
    TokenEmergencyActionTransitionActionV0 {
        base: test_token_base(),
        emergency_action: TokenEmergencyAction::Pause,
        public_note: Some("emergency note".to_string()),
    }
}

fn make_emergency() -> TokenEmergencyActionTransitionAction {
    TokenEmergencyActionTransitionAction::V0(make_emergency_v0())
}

#[test]
fn test_emergency_v0_base() {
    let action = make_emergency_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_emergency_v0_emergency_action() {
    let action = make_emergency_v0();
    assert_eq!(action.emergency_action(), TokenEmergencyAction::Pause);
}

#[test]
fn test_emergency_v0_set_emergency_action() {
    let mut action = make_emergency_v0();
    action.set_emergency_action(TokenEmergencyAction::Resume);
    assert_eq!(action.emergency_action(), TokenEmergencyAction::Resume);
}

#[test]
fn test_emergency_v0_public_note() {
    let action = make_emergency_v0();
    assert_eq!(action.public_note(), Some(&"emergency note".to_string()));
}

#[test]
fn test_emergency_v0_public_note_owned() {
    let action = make_emergency_v0();
    assert_eq!(
        action.public_note_owned(),
        Some("emergency note".to_string())
    );
}

#[test]
fn test_emergency_v0_set_public_note() {
    let mut action = make_emergency_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_emergency_action_base() {
    let action = make_emergency();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_emergency_action_base_owned() {
    let action = make_emergency();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_emergency_action_emergency_action() {
    let action = make_emergency();
    assert_eq!(action.emergency_action(), TokenEmergencyAction::Pause);
}

#[test]
fn test_emergency_action_set_emergency_action() {
    let mut action = make_emergency();
    action.set_emergency_action(TokenEmergencyAction::Resume);
    assert_eq!(action.emergency_action(), TokenEmergencyAction::Resume);
}

#[test]
fn test_emergency_action_public_note() {
    let action = make_emergency();
    assert_eq!(action.public_note(), Some(&"emergency note".to_string()));
}

#[test]
fn test_emergency_action_public_note_owned() {
    let action = make_emergency();
    assert_eq!(
        action.public_note_owned(),
        Some("emergency note".to_string())
    );
}

#[test]
fn test_emergency_action_set_public_note() {
    let mut action = make_emergency();
    action.set_public_note(Some("new".to_string()));
    assert_eq!(action.public_note(), Some(&"new".to_string()));
}

// ============================================================
// 17. TokenDestroyFrozenFundsTransitionAction tests
// ============================================================

fn make_destroy_v0() -> TokenDestroyFrozenFundsTransitionActionV0 {
    TokenDestroyFrozenFundsTransitionActionV0 {
        base: test_token_base(),
        frozen_identity_id: Identifier::from([0xCC; 32]),
        amount: 400,
        public_note: Some("destroy note".to_string()),
    }
}

fn make_destroy() -> TokenDestroyFrozenFundsTransitionAction {
    TokenDestroyFrozenFundsTransitionAction::V0(make_destroy_v0())
}

#[test]
fn test_destroy_v0_base() {
    let action = make_destroy_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_destroy_v0_frozen_identity_id() {
    let action = make_destroy_v0();
    assert_eq!(action.frozen_identity_id(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_destroy_v0_set_frozen_identity_id() {
    let mut action = make_destroy_v0();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_frozen_identity_id(new_id);
    assert_eq!(action.frozen_identity_id(), new_id);
}

#[test]
fn test_destroy_v0_amount() {
    let action = make_destroy_v0();
    assert_eq!(action.amount(), 400);
}

#[test]
fn test_destroy_v0_set_amount() {
    let mut action = make_destroy_v0();
    action.set_amount(800);
    assert_eq!(action.amount(), 800);
}

#[test]
fn test_destroy_v0_public_note() {
    let action = make_destroy_v0();
    assert_eq!(action.public_note(), Some(&"destroy note".to_string()));
}

#[test]
fn test_destroy_v0_public_note_owned() {
    let action = make_destroy_v0();
    assert_eq!(
        action.public_note_owned(),
        Some("destroy note".to_string())
    );
}

#[test]
fn test_destroy_v0_set_public_note() {
    let mut action = make_destroy_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_destroy_action_base() {
    let action = make_destroy();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_destroy_action_base_owned() {
    let action = make_destroy();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_destroy_action_frozen_identity_id() {
    let action = make_destroy();
    assert_eq!(action.frozen_identity_id(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_destroy_action_set_frozen_identity_id() {
    let mut action = make_destroy();
    let new_id = Identifier::from([0xFF; 32]);
    action.set_frozen_identity_id(new_id);
    assert_eq!(action.frozen_identity_id(), new_id);
}

#[test]
fn test_destroy_action_amount() {
    let action = make_destroy();
    assert_eq!(action.amount(), 400);
}

#[test]
fn test_destroy_action_set_amount() {
    let mut action = make_destroy();
    action.set_amount(800);
    assert_eq!(action.amount(), 800);
}

#[test]
fn test_destroy_action_public_note() {
    let action = make_destroy();
    assert_eq!(action.public_note(), Some(&"destroy note".to_string()));
}

#[test]
fn test_destroy_action_public_note_owned() {
    let action = make_destroy();
    assert_eq!(
        action.public_note_owned(),
        Some("destroy note".to_string())
    );
}

#[test]
fn test_destroy_action_set_public_note() {
    let mut action = make_destroy();
    action.set_public_note(Some("new".to_string()));
    assert_eq!(action.public_note(), Some(&"new".to_string()));
}

// ============================================================
// 18. TokenConfigUpdateTransitionAction tests
// ============================================================

fn make_config_update_v0() -> TokenConfigUpdateTransitionActionV0 {
    TokenConfigUpdateTransitionActionV0 {
        base: test_token_base(),
        update_token_configuration_item: TokenConfigurationChangeItem::default(),
        public_note: Some("config note".to_string()),
    }
}

fn make_config_update() -> TokenConfigUpdateTransitionAction {
    TokenConfigUpdateTransitionAction::V0(make_config_update_v0())
}

#[test]
fn test_config_update_v0_base() {
    let action = make_config_update_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_config_update_v0_update_token_configuration_item() {
    let action = make_config_update_v0();
    // Default is TokenConfigurationNoChange
    assert_eq!(
        *action.update_token_configuration_item(),
        TokenConfigurationChangeItem::default()
    );
}

#[test]
fn test_config_update_v0_set_update_token_configuration_item() {
    let mut action = make_config_update_v0();
    action.set_update_token_configuration_item(TokenConfigurationChangeItem::default());
    assert_eq!(
        *action.update_token_configuration_item(),
        TokenConfigurationChangeItem::default()
    );
}

#[test]
fn test_config_update_v0_public_note() {
    let action = make_config_update_v0();
    assert_eq!(action.public_note(), Some(&"config note".to_string()));
}

#[test]
fn test_config_update_v0_public_note_owned() {
    let action = make_config_update_v0();
    assert_eq!(
        action.public_note_owned(),
        Some("config note".to_string())
    );
}

#[test]
fn test_config_update_v0_set_public_note() {
    let mut action = make_config_update_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_config_update_action_base() {
    let action = make_config_update();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_config_update_action_base_owned() {
    let action = make_config_update();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_config_update_action_update_token_configuration_item() {
    let action = make_config_update();
    assert_eq!(
        *action.update_token_configuration_item(),
        TokenConfigurationChangeItem::default()
    );
}

#[test]
fn test_config_update_action_set_update_token_configuration_item() {
    let mut action = make_config_update();
    action.set_update_token_configuration_item(TokenConfigurationChangeItem::default());
    assert_eq!(
        *action.update_token_configuration_item(),
        TokenConfigurationChangeItem::default()
    );
}

#[test]
fn test_config_update_action_public_note() {
    let action = make_config_update();
    assert_eq!(action.public_note(), Some(&"config note".to_string()));
}

#[test]
fn test_config_update_action_public_note_owned() {
    let action = make_config_update();
    assert_eq!(
        action.public_note_owned(),
        Some("config note".to_string())
    );
}

#[test]
fn test_config_update_action_set_public_note() {
    let mut action = make_config_update();
    action.set_public_note(Some("changed".to_string()));
    assert_eq!(action.public_note(), Some(&"changed".to_string()));
}

// ============================================================
// 19. TokenDirectPurchaseTransitionAction tests
// ============================================================

fn make_direct_purchase_v0() -> TokenDirectPurchaseTransitionActionV0 {
    TokenDirectPurchaseTransitionActionV0 {
        base: test_token_base(),
        token_count: 50,
        total_agreed_price: 10_000,
    }
}

fn make_direct_purchase() -> TokenDirectPurchaseTransitionAction {
    TokenDirectPurchaseTransitionAction::V0(make_direct_purchase_v0())
}

#[test]
fn test_direct_purchase_v0_base() {
    let action = make_direct_purchase_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_direct_purchase_v0_token_count() {
    let action = make_direct_purchase_v0();
    assert_eq!(action.token_count(), 50);
}

#[test]
fn test_direct_purchase_v0_set_token_count() {
    let mut action = make_direct_purchase_v0();
    action.set_token_count(100);
    assert_eq!(action.token_count(), 100);
}

#[test]
fn test_direct_purchase_v0_total_agreed_price() {
    let action = make_direct_purchase_v0();
    assert_eq!(action.total_agreed_price(), 10_000);
}

#[test]
fn test_direct_purchase_v0_set_total_agreed_price() {
    let mut action = make_direct_purchase_v0();
    action.set_total_agreed_price(20_000);
    assert_eq!(action.total_agreed_price(), 20_000);
}

// Enum wrapper
#[test]
fn test_direct_purchase_action_base() {
    let action = make_direct_purchase();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_direct_purchase_action_base_owned() {
    let action = make_direct_purchase();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_direct_purchase_action_token_count() {
    let action = make_direct_purchase();
    assert_eq!(action.token_count(), 50);
}

#[test]
fn test_direct_purchase_action_set_token_count() {
    let mut action = make_direct_purchase();
    action.set_token_count(100);
    assert_eq!(action.token_count(), 100);
}

#[test]
fn test_direct_purchase_action_total_agreed_price() {
    let action = make_direct_purchase();
    assert_eq!(action.total_agreed_price(), 10_000);
}

#[test]
fn test_direct_purchase_action_set_total_agreed_price() {
    let mut action = make_direct_purchase();
    action.set_total_agreed_price(20_000);
    assert_eq!(action.total_agreed_price(), 20_000);
}

// ============================================================
// 20. TokenSetPriceForDirectPurchaseTransitionAction tests
// ============================================================

fn make_set_price_v0() -> TokenSetPriceForDirectPurchaseTransitionActionV0 {
    TokenSetPriceForDirectPurchaseTransitionActionV0 {
        base: test_token_base(),
        price: Some(TokenPricingSchedule::SinglePrice(500)),
        public_note: Some("set price note".to_string()),
    }
}

fn make_set_price() -> TokenSetPriceForDirectPurchaseTransitionAction {
    TokenSetPriceForDirectPurchaseTransitionAction::V0(make_set_price_v0())
}

#[test]
fn test_set_price_v0_base() {
    let action = make_set_price_v0();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_set_price_v0_price() {
    let action = make_set_price_v0();
    assert_eq!(
        action.price(),
        Some(&TokenPricingSchedule::SinglePrice(500))
    );
}

#[test]
fn test_set_price_v0_set_price() {
    let mut action = make_set_price_v0();
    action.set_price(None);
    assert!(action.price().is_none());
}

#[test]
fn test_set_price_v0_public_note() {
    let action = make_set_price_v0();
    assert_eq!(action.public_note(), Some(&"set price note".to_string()));
}

#[test]
fn test_set_price_v0_public_note_owned() {
    let action = make_set_price_v0();
    assert_eq!(
        action.public_note_owned(),
        Some("set price note".to_string())
    );
}

#[test]
fn test_set_price_v0_set_public_note() {
    let mut action = make_set_price_v0();
    action.set_public_note(None);
    assert!(action.public_note().is_none());
}

// Enum wrapper
#[test]
fn test_set_price_action_base() {
    let action = make_set_price();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_set_price_action_base_owned() {
    let action = make_set_price();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_set_price_action_price() {
    let action = make_set_price();
    assert_eq!(
        action.price(),
        Some(&TokenPricingSchedule::SinglePrice(500))
    );
}

#[test]
fn test_set_price_action_set_price() {
    let mut action = make_set_price();
    action.set_price(Some(TokenPricingSchedule::SinglePrice(999)));
    assert_eq!(
        action.price(),
        Some(&TokenPricingSchedule::SinglePrice(999))
    );
}

#[test]
fn test_set_price_action_public_note() {
    let action = make_set_price();
    assert_eq!(action.public_note(), Some(&"set price note".to_string()));
}

#[test]
fn test_set_price_action_public_note_owned() {
    let action = make_set_price();
    assert_eq!(
        action.public_note_owned(),
        Some("set price note".to_string())
    );
}

#[test]
fn test_set_price_action_set_public_note() {
    let mut action = make_set_price();
    action.set_public_note(Some("changed".to_string()));
    assert_eq!(action.public_note(), Some(&"changed".to_string()));
}

// ============================================================
// 21. TokenTransitionAction enum tests
// ============================================================

#[test]
fn test_token_transition_action_burn_base() {
    let action: TokenTransitionAction = make_burn().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_burn_base_owned() {
    let action: TokenTransitionAction = make_burn().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_mint_base() {
    let action: TokenTransitionAction = make_mint().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_mint_base_owned() {
    let action: TokenTransitionAction = make_mint().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_transfer_base() {
    let action: TokenTransitionAction = make_token_transfer().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_transfer_base_owned() {
    let action: TokenTransitionAction = make_token_transfer().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_freeze_base() {
    let action: TokenTransitionAction = make_freeze().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_freeze_base_owned() {
    let action: TokenTransitionAction = make_freeze().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_unfreeze_base() {
    let action: TokenTransitionAction = make_unfreeze().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_unfreeze_base_owned() {
    let action: TokenTransitionAction = make_unfreeze().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_claim_base() {
    let action: TokenTransitionAction = make_claim().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_claim_base_owned() {
    let action: TokenTransitionAction = make_claim().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_emergency_base() {
    let action: TokenTransitionAction = make_emergency().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_emergency_base_owned() {
    let action: TokenTransitionAction = make_emergency().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_destroy_base() {
    let action: TokenTransitionAction = make_destroy().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_destroy_base_owned() {
    let action: TokenTransitionAction = make_destroy().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_config_update_base() {
    let action: TokenTransitionAction = make_config_update().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_config_update_base_owned() {
    let action: TokenTransitionAction = make_config_update().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_direct_purchase_base() {
    let action: TokenTransitionAction = make_direct_purchase().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_direct_purchase_base_owned() {
    let action: TokenTransitionAction = make_direct_purchase().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

#[test]
fn test_token_transition_action_set_price_base() {
    let action: TokenTransitionAction = make_set_price().into();
    assert_eq!(action.base().token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_token_transition_action_set_price_base_owned() {
    let action: TokenTransitionAction = make_set_price().into();
    assert_eq!(
        action.base_owned().token_id(),
        Identifier::from([0xBB; 32])
    );
}

// historical_document_type_name tests for all 11 variants
#[test]
fn test_token_transition_action_historical_document_type_name_burn() {
    let action: TokenTransitionAction = make_burn().into();
    assert_eq!(action.historical_document_type_name(), "burn");
}

#[test]
fn test_token_transition_action_historical_document_type_name_mint() {
    let action: TokenTransitionAction = make_mint().into();
    assert_eq!(action.historical_document_type_name(), "mint");
}

#[test]
fn test_token_transition_action_historical_document_type_name_transfer() {
    let action: TokenTransitionAction = make_token_transfer().into();
    assert_eq!(action.historical_document_type_name(), "transfer");
}

#[test]
fn test_token_transition_action_historical_document_type_name_freeze() {
    let action: TokenTransitionAction = make_freeze().into();
    assert_eq!(action.historical_document_type_name(), "freeze");
}

#[test]
fn test_token_transition_action_historical_document_type_name_unfreeze() {
    let action: TokenTransitionAction = make_unfreeze().into();
    assert_eq!(action.historical_document_type_name(), "unfreeze");
}

#[test]
fn test_token_transition_action_historical_document_type_name_claim() {
    let action: TokenTransitionAction = make_claim().into();
    assert_eq!(action.historical_document_type_name(), "claim");
}

#[test]
fn test_token_transition_action_historical_document_type_name_emergency() {
    let action: TokenTransitionAction = make_emergency().into();
    assert_eq!(action.historical_document_type_name(), "emergencyAction");
}

#[test]
fn test_token_transition_action_historical_document_type_name_destroy() {
    let action: TokenTransitionAction = make_destroy().into();
    assert_eq!(
        action.historical_document_type_name(),
        "destroyFrozenFunds"
    );
}

#[test]
fn test_token_transition_action_historical_document_type_name_config_update() {
    let action: TokenTransitionAction = make_config_update().into();
    assert_eq!(action.historical_document_type_name(), "configUpdate");
}

#[test]
fn test_token_transition_action_historical_document_type_name_direct_purchase() {
    let action: TokenTransitionAction = make_direct_purchase().into();
    assert_eq!(action.historical_document_type_name(), "directPurchase");
}

#[test]
fn test_token_transition_action_historical_document_type_name_set_price() {
    let action: TokenTransitionAction = make_set_price().into();
    assert_eq!(action.historical_document_type_name(), "directPricing");
}

// ============================================================
// 22. BatchedTransitionAction tests
// ============================================================

fn make_bump_action() -> BumpIdentityDataContractNonceAction {
    BumpIdentityDataContractNonceAction::V0(BumpIdentityDataContractNonceActionV0 {
        identity_id: Identifier::from([0x11; 32]),
        data_contract_id: Identifier::from([0x22; 32]),
        identity_contract_nonce: 5,
        user_fee_increase: 0,
    })
}

#[test]
fn test_batched_transition_action_data_contract_id_document() {
    let doc_action: DocumentTransitionAction = make_create().into();
    let batched = BatchedTransitionAction::DocumentAction(doc_action);
    let contract_id = batched.data_contract_id();
    // Verify that contract_id is a non-default identifier (DPNS contract)
    assert_ne!(contract_id, Identifier::default());
}

#[test]
fn test_batched_transition_action_data_contract_id_token() {
    let token_action: TokenTransitionAction = make_burn().into();
    let batched = BatchedTransitionAction::TokenAction(token_action);
    let contract_id = batched.data_contract_id();
    assert_ne!(contract_id, Identifier::default());
}

#[test]
fn test_batched_transition_action_data_contract_id_bump() {
    let batched = BatchedTransitionAction::BumpIdentityDataContractNonce(make_bump_action());
    assert_eq!(batched.data_contract_id(), Identifier::from([0x22; 32]));
}

#[test]
fn test_batched_transition_action_as_document_action_ok() {
    let doc_action: DocumentTransitionAction = make_create().into();
    let batched = BatchedTransitionAction::DocumentAction(doc_action);
    assert!(batched.as_document_action().is_ok());
}

#[test]
fn test_batched_transition_action_as_document_action_err_for_token() {
    let token_action: TokenTransitionAction = make_burn().into();
    let batched = BatchedTransitionAction::TokenAction(token_action);
    assert!(batched.as_document_action().is_err());
}

#[test]
fn test_batched_transition_action_as_document_action_err_for_bump() {
    let batched = BatchedTransitionAction::BumpIdentityDataContractNonce(make_bump_action());
    assert!(batched.as_document_action().is_err());
}

#[test]
fn test_batched_transition_action_as_token_action_ok() {
    let token_action: TokenTransitionAction = make_burn().into();
    let batched = BatchedTransitionAction::TokenAction(token_action);
    assert!(batched.as_token_action().is_ok());
}

#[test]
fn test_batched_transition_action_as_token_action_err_for_document() {
    let doc_action: DocumentTransitionAction = make_create().into();
    let batched = BatchedTransitionAction::DocumentAction(doc_action);
    assert!(batched.as_token_action().is_err());
}

#[test]
fn test_batched_transition_action_as_token_action_err_for_bump() {
    let batched = BatchedTransitionAction::BumpIdentityDataContractNonce(make_bump_action());
    assert!(batched.as_token_action().is_err());
}

#[test]
fn test_batched_transition_action_as_bump_identity_nonce_action_ok() {
    let batched = BatchedTransitionAction::BumpIdentityDataContractNonce(make_bump_action());
    assert!(batched.as_bump_identity_nonce_action().is_ok());
}

#[test]
fn test_batched_transition_action_as_bump_identity_nonce_action_err_for_document() {
    let doc_action: DocumentTransitionAction = make_create().into();
    let batched = BatchedTransitionAction::DocumentAction(doc_action);
    assert!(batched.as_bump_identity_nonce_action().is_err());
}

#[test]
fn test_batched_transition_action_as_bump_identity_nonce_action_err_for_token() {
    let token_action: TokenTransitionAction = make_burn().into();
    let batched = BatchedTransitionAction::TokenAction(token_action);
    assert!(batched.as_bump_identity_nonce_action().is_err());
}

// ============================================================
// 23. BatchTransitionAction tests
// ============================================================

fn make_batch_v0() -> BatchTransitionActionV0 {
    BatchTransitionActionV0 {
        owner_id: Identifier::from([0x11; 32]),
        transitions: vec![],
        user_fee_increase: 10,
    }
}

fn make_batch() -> BatchTransitionAction {
    BatchTransitionAction::V0(make_batch_v0())
}

#[test]
fn test_batch_action_owner_id() {
    let batch = make_batch();
    assert_eq!(batch.owner_id(), Identifier::from([0x11; 32]));
}

#[test]
fn test_batch_action_transitions_empty() {
    let batch = make_batch();
    assert!(batch.transitions().is_empty());
}

#[test]
fn test_batch_action_transitions_mut() {
    let mut batch = make_batch();
    let doc_action: DocumentTransitionAction = make_delete().into();
    batch
        .transitions_mut()
        .push(BatchedTransitionAction::DocumentAction(doc_action));
    assert_eq!(batch.transitions().len(), 1);
}

#[test]
fn test_batch_action_transitions_take() {
    let mut batch = make_batch();
    let doc_action: DocumentTransitionAction = make_delete().into();
    batch
        .transitions_mut()
        .push(BatchedTransitionAction::DocumentAction(doc_action));
    let taken = batch.transitions_take();
    assert_eq!(taken.len(), 1);
    assert!(batch.transitions().is_empty());
}

#[test]
fn test_batch_action_transitions_owned() {
    let mut batch = make_batch();
    let doc_action: DocumentTransitionAction = make_delete().into();
    batch
        .transitions_mut()
        .push(BatchedTransitionAction::DocumentAction(doc_action));
    let owned = batch.transitions_owned();
    assert_eq!(owned.len(), 1);
}

#[test]
fn test_batch_action_set_transitions() {
    let mut batch = make_batch();
    let doc_action: DocumentTransitionAction = make_delete().into();
    batch.set_transitions(vec![BatchedTransitionAction::DocumentAction(doc_action)]);
    assert_eq!(batch.transitions().len(), 1);
}

#[test]
fn test_batch_action_user_fee_increase() {
    let batch = make_batch();
    assert_eq!(batch.user_fee_increase(), 10);
}

// ============================================================
// 24. BatchTransitionActionV0 computed methods
// ============================================================

#[test]
fn test_all_purchases_amount_no_purchases() {
    let batch = make_batch();
    let result = batch.all_purchases_amount().unwrap();
    assert!(result.is_none());
}

#[test]
fn test_all_purchases_amount_one_document_purchase() {
    let mut batch = make_batch();
    let purchase_action: DocumentTransitionAction = make_purchase().into();
    batch.set_transitions(vec![BatchedTransitionAction::DocumentAction(
        purchase_action,
    )]);
    let result = batch.all_purchases_amount().unwrap();
    assert_eq!(result, Some(5000));
}

#[test]
fn test_all_purchases_amount_multiple_document_purchases() {
    let mut batch = make_batch();
    let p1: DocumentTransitionAction = make_purchase().into();
    let p2: DocumentTransitionAction =
        DocumentPurchaseTransitionAction::V0(DocumentPurchaseTransitionActionV0 {
            base: test_document_base(),
            document: test_document(),
            original_owner_id: Identifier::from([0xDD; 32]),
            price: 3000,
        })
        .into();
    batch.set_transitions(vec![
        BatchedTransitionAction::DocumentAction(p1),
        BatchedTransitionAction::DocumentAction(p2),
    ]);
    let result = batch.all_purchases_amount().unwrap();
    assert_eq!(result, Some(8000)); // 5000 + 3000
}

#[test]
fn test_all_purchases_amount_one_token_direct_purchase() {
    let mut batch = make_batch();
    let token_purchase: TokenTransitionAction = make_direct_purchase().into();
    batch.set_transitions(vec![BatchedTransitionAction::TokenAction(token_purchase)]);
    let result = batch.all_purchases_amount().unwrap();
    assert_eq!(result, Some(10_000));
}

#[test]
fn test_all_purchases_amount_mixed_purchases() {
    let mut batch = make_batch();
    let doc_purchase: DocumentTransitionAction = make_purchase().into();
    let token_purchase: TokenTransitionAction = make_direct_purchase().into();
    batch.set_transitions(vec![
        BatchedTransitionAction::DocumentAction(doc_purchase),
        BatchedTransitionAction::TokenAction(token_purchase),
    ]);
    let result = batch.all_purchases_amount().unwrap();
    assert_eq!(result, Some(15_000)); // 5000 + 10_000
}

#[test]
fn test_all_conflicting_index_collateral_voting_funds_no_creates() {
    let batch = make_batch();
    let result = batch
        .all_conflicting_index_collateral_voting_funds()
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_all_conflicting_index_collateral_voting_funds_create_without_prefunded() {
    let mut batch = make_batch();
    let create: DocumentTransitionAction = make_create().into();
    batch.set_transitions(vec![BatchedTransitionAction::DocumentAction(create)]);
    // A CreateAction with prefunded_voting_balance = None still yields Some(0)
    // because iter() on &None yields an empty iterator, and try_fold returns Some(0).
    let result = batch
        .all_conflicting_index_collateral_voting_funds()
        .unwrap();
    assert_eq!(result, Some(0));
}

#[test]
fn test_all_used_balances_none() {
    let batch = make_batch();
    let result = batch.all_used_balances().unwrap();
    assert!(result.is_none());
}

#[test]
fn test_all_used_balances_purchases_only() {
    let mut batch = make_batch();
    let purchase: DocumentTransitionAction = make_purchase().into();
    batch.set_transitions(vec![BatchedTransitionAction::DocumentAction(purchase)]);
    let result = batch.all_used_balances().unwrap();
    assert_eq!(result, Some(5000));
}

#[test]
fn test_all_used_balances_non_purchase_transitions_ignored() {
    let mut batch = make_batch();
    let delete: DocumentTransitionAction = make_delete().into();
    let create: DocumentTransitionAction = make_create().into();
    batch.set_transitions(vec![
        BatchedTransitionAction::DocumentAction(delete),
        BatchedTransitionAction::DocumentAction(create),
    ]);
    let result = batch.all_used_balances().unwrap();
    // delete contributes nothing, create with no prefunded balance yields Some(0)
    // for voting funds, so all_used_balances returns Some(0)
    assert_eq!(result, Some(0));
}

// ============================================================
// 25. From conversion tests
// ============================================================

#[test]
fn test_document_base_from_v0() {
    let v0 = test_document_base_v0();
    let action: DocumentBaseTransitionAction = v0.into();
    assert_eq!(action.id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_token_base_from_v0() {
    let v0 = test_token_base_v0();
    let action: TokenBaseTransitionAction = v0.into();
    assert_eq!(action.token_id(), Identifier::from([0xBB; 32]));
}

#[test]
fn test_document_create_from_v0() {
    let v0 = make_create_v0();
    let action: DocumentCreateTransitionAction = v0.into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_delete_from_v0() {
    let v0 = make_delete_v0();
    let action: DocumentDeleteTransitionAction = v0.into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_replace_from_v0() {
    let v0 = make_replace_v0();
    let action: DocumentReplaceTransitionAction = v0.into();
    assert_eq!(action.revision(), 2);
}

#[test]
fn test_document_transfer_from_v0() {
    let v0 = make_transfer_v0();
    let action: DocumentTransferTransitionAction = v0.into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_document_purchase_from_v0() {
    let v0 = make_purchase_v0();
    let action: DocumentPurchaseTransitionAction = v0.into();
    assert_eq!(action.price(), 5000);
}

#[test]
fn test_document_update_price_from_v0() {
    let v0 = make_update_price_v0();
    let action: DocumentUpdatePriceTransitionAction = v0.into();
    assert_eq!(action.base().id(), Identifier::from([0xAA; 32]));
}

#[test]
fn test_token_burn_from_v0() {
    let v0 = make_burn_v0();
    let action: TokenBurnTransitionAction = v0.into();
    assert_eq!(action.burn_amount(), 500);
}

#[test]
fn test_token_mint_from_v0() {
    let v0 = make_mint_v0();
    let action: TokenMintTransitionAction = v0.into();
    assert_eq!(action.mint_amount(), 1000);
}

#[test]
fn test_token_transfer_from_v0() {
    let v0 = make_token_transfer_v0();
    let action: TokenTransferTransitionAction = v0.into();
    assert_eq!(action.amount(), 750);
}

#[test]
fn test_token_freeze_from_v0() {
    let v0 = make_freeze_v0();
    let action: TokenFreezeTransitionAction = v0.into();
    assert_eq!(
        action.identity_to_freeze_id(),
        Identifier::from([0xCC; 32])
    );
}

#[test]
fn test_token_unfreeze_from_v0() {
    let v0 = make_unfreeze_v0();
    let action: TokenUnfreezeTransitionAction = v0.into();
    assert_eq!(action.frozen_identity_id(), Identifier::from([0xCC; 32]));
}

#[test]
fn test_token_claim_from_v0() {
    let v0 = make_claim_v0();
    let action: TokenClaimTransitionAction = v0.into();
    assert_eq!(action.amount(), 300);
}

#[test]
fn test_token_emergency_from_v0() {
    let v0 = make_emergency_v0();
    let action: TokenEmergencyActionTransitionAction = v0.into();
    assert_eq!(action.emergency_action(), TokenEmergencyAction::Pause);
}

#[test]
fn test_token_destroy_from_v0() {
    let v0 = make_destroy_v0();
    let action: TokenDestroyFrozenFundsTransitionAction = v0.into();
    assert_eq!(action.amount(), 400);
}

#[test]
fn test_token_config_update_from_v0() {
    let v0 = make_config_update_v0();
    let action: TokenConfigUpdateTransitionAction = v0.into();
    assert_eq!(
        *action.update_token_configuration_item(),
        TokenConfigurationChangeItem::default()
    );
}

#[test]
fn test_token_direct_purchase_from_v0() {
    let v0 = make_direct_purchase_v0();
    let action: TokenDirectPurchaseTransitionAction = v0.into();
    assert_eq!(action.token_count(), 50);
}

#[test]
fn test_token_set_price_from_v0() {
    let v0 = make_set_price_v0();
    let action: TokenSetPriceForDirectPurchaseTransitionAction = v0.into();
    assert_eq!(
        action.price(),
        Some(&TokenPricingSchedule::SinglePrice(500))
    );
}

#[test]
fn test_batch_transition_action_from_v0() {
    let v0 = make_batch_v0();
    let action: BatchTransitionAction = v0.into();
    assert_eq!(action.owner_id(), Identifier::from([0x11; 32]));
}

// ============================================================
// 26. BatchedTransitionAction From conversion tests
// ============================================================

#[test]
fn test_batched_transition_from_document_transition_action() {
    let doc_action: DocumentTransitionAction = make_create().into();
    let batched: BatchedTransitionAction = doc_action.into();
    assert!(batched.as_document_action().is_ok());
}

#[test]
fn test_batched_transition_from_token_transition_action() {
    let token_action: TokenTransitionAction = make_burn().into();
    let batched: BatchedTransitionAction = token_action.into();
    assert!(batched.as_token_action().is_ok());
}

#[test]
fn test_batched_transition_from_bump_action() {
    let bump = make_bump_action();
    let batched: BatchedTransitionAction = bump.into();
    assert!(batched.as_bump_identity_nonce_action().is_ok());
}
