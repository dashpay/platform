//! Batch action format 1 must agree with format 0 on every item format 0 can
//! express, and treat an erase as a document action that moves no credits.

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::identity::SecurityLevel;
use dpp::platform_value::platform_value;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::version::PlatformVersion;
use grovedb_costs::OperationCost;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
    DocumentBaseTransitionAction, DocumentBaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::DocumentEraseTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::DocumentEraseTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::{
    DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::{
    TokenDirectPurchaseTransitionAction, TokenDirectPurchaseTransitionActionV0,
};
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::v0::BatchTransitionActionV0;
use crate::state_transition_action::batch::v1::{BatchTransitionActionV1, BatchedTransitionActionV1};
use crate::state_transition_action::batch::BatchTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
    BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0,
};

const OWNER_ID: [u8; 32] = [0x11; 32];
const DOCUMENT_ID: [u8; 32] = [0xAA; 32];
const PURCHASE_PRICE: u64 = 5_000;
const TOKEN_PRICE: u64 = 10_000;

fn dpns_contract_info() -> Arc<DataContractFetchInfo> {
    Arc::new(DataContractFetchInfo::dpns_contract_fixture(
        PlatformVersion::latest().protocol_version,
    ))
}

/// The DPNS contract with one extra document type whose signature must come
/// from a master key.
fn contract_info_with_master_level_type() -> Arc<DataContractFetchInfo> {
    let platform_version = PlatformVersion::latest();
    let mut contract = dpns_contract_info().contract.clone();
    let config = *contract.config();
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "position": 0,
            }
        },
        "signatureSecurityLevelRequirement": SecurityLevel::MASTER as u8,
        "additionalProperties": false,
    });
    let document_type = DocumentType::try_from_schema(
        contract.id(),
        1,
        config.version(),
        "vault",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        false,
        &mut Vec::new(),
        platform_version,
    )
    .expect("a document type with a master-level signature requirement");
    contract
        .document_types_mut()
        .insert("vault".to_string(), document_type);
    Arc::new(DataContractFetchInfo {
        contract,
        storage_flags: None,
        cost: OperationCost::default(),
        fee: None,
    })
}

fn document_base(
    document_type_name: &str,
    data_contract: Arc<DataContractFetchInfo>,
) -> DocumentBaseTransitionAction {
    DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
        id: Identifier::from(DOCUMENT_ID),
        identity_contract_nonce: 1,
        document_type_name: document_type_name.to_string(),
        data_contract,
        token_cost: None,
        gas_fees_paid_by: GasFeesPaidBy::default(),
    })
}

fn erase(
    document_type_name: &str,
    data_contract: Arc<DataContractFetchInfo>,
) -> BatchedTransitionActionV1 {
    BatchedTransitionActionV1::DocumentErase(DocumentEraseTransitionAction::V0(
        DocumentEraseTransitionActionV0 {
            base: document_base(document_type_name, data_contract),
        },
    ))
}

fn domain_delete() -> BatchedTransitionAction {
    BatchedTransitionAction::DocumentAction(DocumentTransitionAction::DeleteAction(
        DocumentDeleteTransitionAction::V0(DocumentDeleteTransitionActionV0 {
            base: document_base("domain", dpns_contract_info()),
        }),
    ))
}

fn domain_purchase() -> BatchedTransitionAction {
    BatchedTransitionAction::DocumentAction(DocumentTransitionAction::PurchaseAction(
        DocumentPurchaseTransitionAction::V0(DocumentPurchaseTransitionActionV0 {
            base: document_base("domain", dpns_contract_info()),
            document: Document::V0(DocumentV0::default()),
            original_owner_id: Identifier::from([0xDD; 32]),
            price: PURCHASE_PRICE,
        }),
    ))
}

fn token_purchase() -> BatchedTransitionAction {
    BatchedTransitionAction::TokenAction(TokenTransitionAction::DirectPurchaseAction(
        TokenDirectPurchaseTransitionAction::V0(TokenDirectPurchaseTransitionActionV0 {
            base: TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
                token_id: Identifier::from([0xBB; 32]),
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract: dpns_contract_info(),
                store_in_group: None,
                perform_action: true,
            }),
            token_count: 50,
            total_agreed_price: TOKEN_PRICE,
        }),
    ))
}

fn nonce_bump() -> BatchedTransitionAction {
    BatchedTransitionAction::BumpIdentityDataContractNonce(BumpIdentityDataContractNonceAction::V0(
        BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from(OWNER_ID),
            data_contract_id: Identifier::from([0x22; 32]),
            identity_contract_nonce: 5,
            user_fee_increase: 0,
        },
    ))
}

fn format_0_batch(transitions: Vec<BatchedTransitionAction>) -> BatchTransitionAction {
    BatchTransitionAction::V0(BatchTransitionActionV0 {
        owner_id: Identifier::from(OWNER_ID),
        transitions,
        user_fee_increase: 10,
    })
}

fn format_1_batch(transitions: Vec<BatchedTransitionActionV1>) -> BatchTransitionActionV1 {
    BatchTransitionActionV1 {
        owner_id: Identifier::from(OWNER_ID),
        transitions,
        user_fee_increase: 10,
    }
}

#[test]
fn should_upcast_a_format_0_batch_keeping_its_items_in_order() {
    let upcast: BatchTransitionActionV1 =
        format_0_batch(vec![domain_delete(), token_purchase(), nonce_bump()]).into();

    assert_eq!(upcast.owner_id(), Identifier::from(OWNER_ID));
    assert_eq!(upcast.user_fee_increase(), 10);
    let kinds: Vec<_> = upcast
        .transitions()
        .iter()
        .map(|item| match item {
            BatchedTransitionActionV1::Batched(BatchedTransitionAction::DocumentAction(_)) => {
                "document"
            }
            BatchedTransitionActionV1::Batched(BatchedTransitionAction::TokenAction(_)) => "token",
            BatchedTransitionActionV1::Batched(
                BatchedTransitionAction::BumpIdentityDataContractNonce(_),
            ) => "bump",
            BatchedTransitionActionV1::DocumentErase(_) => "erase",
        })
        .collect();
    assert_eq!(kinds, ["document", "token", "bump"]);
}

/// The credits a batch commits up front decide whether the signer can afford
/// it; an erase commits none, so its presence must not change the sums.
#[test]
fn should_sum_the_same_credits_as_format_0_with_an_erase_alongside() {
    let format_0 = format_0_batch(vec![domain_purchase(), token_purchase()]);
    let with_erase = format_1_batch(vec![
        erase("domain", dpns_contract_info()),
        BatchedTransitionActionV1::Batched(domain_purchase()),
        BatchedTransitionActionV1::Batched(token_purchase()),
    ]);

    assert_eq!(
        with_erase.all_purchases_amount().unwrap(),
        format_0.all_purchases_amount().unwrap()
    );
    assert_eq!(
        with_erase.all_purchases_amount().unwrap(),
        Some(PURCHASE_PRICE + TOKEN_PRICE)
    );
    assert_eq!(
        with_erase
            .all_conflicting_index_collateral_voting_funds()
            .unwrap(),
        format_0
            .all_conflicting_index_collateral_voting_funds()
            .unwrap()
    );
    assert_eq!(
        with_erase.all_used_balances().unwrap(),
        format_0.all_used_balances().unwrap()
    );

    let erase_only = format_1_batch(vec![erase("domain", dpns_contract_info())]);
    assert_eq!(erase_only.all_purchases_amount().unwrap(), None);
    assert_eq!(
        erase_only
            .all_conflicting_index_collateral_voting_funds()
            .unwrap(),
        None
    );
    assert_eq!(erase_only.all_used_balances().unwrap(), None);
}

/// An erase is signed like any other action on its document type.
#[test]
fn should_require_the_erased_types_security_level() {
    let domain_level = dpns_contract_info()
        .contract
        .document_type_for_name("domain")
        .expect("the DPNS contract has a domain type")
        .security_level_requirement();
    assert_ne!(
        domain_level,
        SecurityLevel::MASTER,
        "the test needs a type whose requirement spans more than one level"
    );

    let erase_only = format_1_batch(vec![erase("domain", dpns_contract_info())]);
    let delete_only = format_0_batch(vec![domain_delete()]);

    let expected: Vec<SecurityLevel> = (SecurityLevel::CRITICAL as u8..=domain_level as u8)
        .map(|level| SecurityLevel::try_from(level).unwrap())
        .collect();
    assert_eq!(
        erase_only.combined_security_level_requirement().unwrap(),
        expected
    );
    assert_eq!(
        erase_only.combined_security_level_requirement().unwrap(),
        delete_only.combined_security_level_requirement().unwrap(),
        "an erase and a delete of the same type demand the same key"
    );
}

#[test]
fn should_require_a_critical_key_for_a_token_action() {
    let batch = format_1_batch(vec![
        erase("domain", dpns_contract_info()),
        BatchedTransitionActionV1::Batched(token_purchase()),
    ]);
    assert_eq!(
        batch.combined_security_level_requirement().unwrap(),
        vec![SecurityLevel::CRITICAL]
    );
}

#[test]
fn should_require_a_master_key_alone_when_an_erased_type_demands_it() {
    let batch = format_1_batch(vec![
        BatchedTransitionActionV1::Batched(domain_delete()),
        erase("vault", contract_info_with_master_level_type()),
    ]);
    assert_eq!(
        batch.combined_security_level_requirement().unwrap(),
        vec![SecurityLevel::MASTER]
    );
}

#[test]
fn should_agree_with_format_0_on_a_batch_of_nonce_bumps() {
    let format_0 = format_0_batch(vec![nonce_bump(), nonce_bump()]);
    let format_1: BatchTransitionActionV1 = format_0.clone().into();
    assert_eq!(
        format_1.combined_security_level_requirement().unwrap(),
        format_0.combined_security_level_requirement().unwrap()
    );
}
