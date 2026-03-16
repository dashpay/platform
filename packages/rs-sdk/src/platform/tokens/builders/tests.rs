use super::burn::TokenBurnTransitionBuilder;
use super::claim::TokenClaimTransitionBuilder;
use super::config_update::TokenConfigUpdateTransitionBuilder;
use super::destroy::TokenDestroyFrozenFundsTransitionBuilder;
use super::emergency_action::TokenEmergencyActionTransitionBuilder;
use super::freeze::TokenFreezeTransitionBuilder;
use super::mint::TokenMintTransitionBuilder;
use super::purchase::TokenDirectPurchaseTransitionBuilder;
use super::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;
use super::transfer::TokenTransferTransitionBuilder;
use super::unfreeze::TokenUnfreezeTransitionBuilder;
use crate::platform::test_helpers::{
    new_mock_sdk_with_contract_nonce, test_data_contract, test_identity_public_key,
    validate_transition_like_builder, TestSigner, INVALID_NONCE, TEST_DOCUMENT_TYPE_NAME,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::ProtocolError;
use std::sync::Arc;

const TEST_TOKEN_POSITION: u16 = 0;

fn token_setup() -> (
    Arc<dpp::data_contract::DataContract>,
    Identifier,
    Identifier,
) {
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let owner_id = Identifier::random();
    let token_id = Identifier::from(calculate_token_id(
        data_contract.id().as_bytes(),
        TEST_TOKEN_POSITION,
    ));
    (data_contract, owner_id, token_id)
}

pub(super) fn assert_token_burn_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_burn_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        1,
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_claim_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_claim_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        TokenDistributionType::PreProgrammed,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_config_update_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_config_update_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        TokenConfigurationChangeItem::TokenConfigurationNoChange,
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_destroy_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_destroy_frozen_funds_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        Identifier::random(),
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_emergency_action_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_emergency_action_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        TokenEmergencyAction::Pause,
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_freeze_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_freeze_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        Identifier::random(),
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_mint_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_mint_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        1,
        Some(Identifier::random()),
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_purchase_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_direct_purchase_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        1,
        10,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_set_price_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_change_direct_purchase_price_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        Some(TokenPricingSchedule::SinglePrice(5)),
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_transfer_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_transfer_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        1,
        Identifier::random(),
        None,
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

pub(super) fn assert_token_unfreeze_validate_base_structure_error() {
    let platform_version = dpp::version::PlatformVersion::latest();
    let (data_contract, owner_id, token_id) = token_setup();
    let transition = BatchTransition::new_token_unfreeze_transition(
        token_id,
        owner_id,
        data_contract.id(),
        TEST_TOKEN_POSITION,
        Identifier::random(),
        None,
        None,
        &test_identity_public_key(),
        INVALID_NONCE,
        0,
        &TestSigner,
        platform_version,
        None,
    )
    .expect("transition should build");

    let result = validate_transition_like_builder(&transition);
    assert!(result.is_err(), "expected validation error, got {result:?}");
}

#[tokio::test]
async fn token_mint_sign_returns_invalid_token_amount_error_when_amount_is_zero() {
    let issuer_id = Identifier::random();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let sdk = new_mock_sdk_with_contract_nonce(issuer_id, data_contract.id(), 0).await;

    let result = TokenMintTransitionBuilder::new(Arc::clone(&data_contract), 0, issuer_id, 0)
        .issued_to_identity_id(Identifier::random())
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_mint_sign_returns_invalid_action_id_error_for_mismatched_group_action_id() {
    let issuer_id = Identifier::random();
    let data_contract = test_data_contract(TEST_DOCUMENT_TYPE_NAME);
    let sdk = new_mock_sdk_with_contract_nonce(issuer_id, data_contract.id(), 0).await;

    let invalid_group_info = GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
        GroupStateTransitionInfo {
            group_contract_position: 0,
            action_id: Identifier::from_bytes(&[0; 32]).expect("create static action id"),
            action_is_proposer: true,
        },
    );

    let result = TokenMintTransitionBuilder::new(Arc::clone(&data_contract), 0, issuer_id, 1)
        .issued_to_identity_id(Identifier::random())
        .with_using_group_info(invalid_group_info)
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidActionIdError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_burn_sign_returns_invalid_token_amount_error_when_amount_is_zero() {
    let owner_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let result = TokenBurnTransitionBuilder::new(Arc::clone(&data_contract), 0, owner_id, 0)
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_burn_sign_returns_note_too_big_error() {
    let owner_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let result = TokenBurnTransitionBuilder::new(Arc::clone(&data_contract), 0, owner_id, 1)
        .with_public_note("x".repeat(2049))
        .sign(
            &sdk,
            &test_identity_public_key(),
            &TestSigner,
            dpp::version::PlatformVersion::latest(),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_transfer_sign_returns_invalid_token_amount_error_when_amount_is_zero() {
    let sender_id = Identifier::random();
    let recipient_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(sender_id, data_contract.id(), 0).await;

    let result = TokenTransferTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        sender_id,
        recipient_id,
        0,
    )
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_transfer_sign_returns_transfer_to_ourself_error() {
    let sender_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(sender_id, data_contract.id(), 0).await;

    let result =
        TokenTransferTransitionBuilder::new(Arc::clone(&data_contract), 0, sender_id, sender_id, 1)
            .sign(
                &sdk,
                &test_identity_public_key(),
                &TestSigner,
                dpp::version::PlatformVersion::latest(),
            )
            .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::TokenTransferToOurselfError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_transfer_sign_returns_note_too_big_error_for_public_note() {
    let sender_id = Identifier::random();
    let recipient_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(sender_id, data_contract.id(), 0).await;

    let result = TokenTransferTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        sender_id,
        recipient_id,
        1,
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_freeze_sign_returns_note_too_big_error() {
    let actor_id = Identifier::random();
    let freeze_identity_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(actor_id, data_contract.id(), 0).await;

    let result = TokenFreezeTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        actor_id,
        freeze_identity_id,
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_unfreeze_sign_returns_note_too_big_error() {
    let actor_id = Identifier::random();
    let unfreeze_identity_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(actor_id, data_contract.id(), 0).await;

    let result = TokenUnfreezeTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        actor_id,
        unfreeze_identity_id,
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_destroy_sign_returns_note_too_big_error() {
    let actor_id = Identifier::random();
    let frozen_identity_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(actor_id, data_contract.id(), 0).await;

    let result = TokenDestroyFrozenFundsTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        actor_id,
        frozen_identity_id,
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_emergency_action_sign_returns_note_too_big_error() {
    let actor_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(actor_id, data_contract.id(), 0).await;

    let result =
        TokenEmergencyActionTransitionBuilder::pause(Arc::clone(&data_contract), 0, actor_id)
            .with_public_note("x".repeat(2049))
            .sign(
                &sdk,
                &test_identity_public_key(),
                &TestSigner,
                dpp::version::PlatformVersion::latest(),
            )
            .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_config_update_sign_returns_no_change_error() {
    let owner_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let result = TokenConfigUpdateTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        owner_id,
        TokenConfigurationChangeItem::TokenConfigurationNoChange,
    )
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenConfigUpdateNoChangeError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_config_update_sign_returns_note_too_big_error() {
    let owner_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let result = TokenConfigUpdateTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        owner_id,
        TokenConfigurationChangeItem::MintingAllowChoosingDestination(true),
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_claim_sign_returns_note_too_big_error() {
    let owner_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(owner_id, data_contract.id(), 0).await;

    let result = TokenClaimTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        owner_id,
        TokenDistributionType::PreProgrammed,
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_purchase_sign_returns_invalid_token_amount_error_when_amount_is_zero() {
    let actor_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(actor_id, data_contract.id(), 0).await;

    let result =
        TokenDirectPurchaseTransitionBuilder::new(Arc::clone(&data_contract), 0, actor_id, 0, 1000)
            .sign(
                &sdk,
                &test_identity_public_key(),
                &TestSigner,
                dpp::version::PlatformVersion::latest(),
            )
            .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}

#[tokio::test]
async fn token_set_price_sign_returns_note_too_big_error() {
    let issuer_id = Identifier::random();
    let data_contract = test_data_contract("testDoc");
    let sdk = new_mock_sdk_with_contract_nonce(issuer_id, data_contract.id(), 0).await;

    let result = TokenChangeDirectPurchasePriceTransitionBuilder::new(
        Arc::clone(&data_contract),
        0,
        issuer_id,
    )
    .with_public_note("x".repeat(2049))
    .sign(
        &sdk,
        &test_identity_public_key(),
        &TestSigner,
        dpp::version::PlatformVersion::latest(),
    )
    .await;

    assert!(
        matches!(
            result,
            Err(crate::Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}
