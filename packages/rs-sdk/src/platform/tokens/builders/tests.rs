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
use crate::{Error, Sdk, SdkBuilder};
use dpp::address_funds::AddressWitness;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContractFactory;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{platform_value, BinaryData, Value};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::ProtocolError;
use drive_proof_verifier::types::IdentityContractNonceFetcher;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug)]
struct TestSigner;

impl Signer<IdentityPublicKey> for TestSigner {
    fn sign(&self, _key: &IdentityPublicKey, _data: &[u8]) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::from(vec![1; 65]))
    }

    fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::CorruptedCodeExecution(
            "sign_create_witness is not used in these tests".to_string(),
        ))
    }

    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

fn test_identity_public_key() -> IdentityPublicKey {
    IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::CRITICAL,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::from(vec![2; 33]),
        disabled_at: None,
    })
}

fn test_data_contract(document_type_name: &str) -> Arc<dpp::data_contract::DataContract> {
    let platform_version = dpp::version::PlatformVersion::latest();
    let config =
        DataContractConfig::default_for_version(platform_version).expect("create contract config");

    let schema = platform_value!({
        "type": "object",
        "properties": {
            "a": {
                "type": "string",
                "maxLength": 10,
                "position": 0
            }
        },
        "additionalProperties": false,
    });

    let document_type = DocumentType::try_from_schema(
        Identifier::random(),
        1,
        config.version(),
        document_type_name,
        schema,
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("create test document type");

    let mut document_types: BTreeMap<String, Value> = BTreeMap::new();
    document_types.insert(
        document_type.name().to_string(),
        document_type.schema().clone(),
    );

    let contract = DataContractFactory::new(platform_version.protocol_version)
        .expect("create data contract factory")
        .create(
            Identifier::random(),
            0,
            platform_value!(document_types),
            None,
            None,
        )
        .expect("create test data contract")
        .data_contract_owned();

    Arc::new(contract)
}

const TEST_DOCUMENT_TYPE_NAME: &str = "testDoc";
const TEST_TOKEN_POSITION: u16 = 0;
const INVALID_NONCE: u64 = 1_u64 << 50;

fn validate_transition_like_builder(state_transition: &StateTransition) -> Result<(), Error> {
    let platform_version = dpp::version::PlatformVersion::latest();
    let validation_result = match state_transition {
        StateTransition::Batch(batch_transition) => {
            batch_transition.validate_base_structure(platform_version)?
        }
        _ => {
            return Err(Error::Protocol(
                dpp::ProtocolError::InvalidStateTransitionType(
                    "expected Batch transition".to_string(),
                ),
            ))
        }
    };
    if let Some(first_error) = validation_result.errors.into_iter().next() {
        return Err(Error::Protocol(dpp::ProtocolError::ConsensusError(
            Box::new(first_error),
        )));
    }
    Ok(())
}

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

async fn new_mock_sdk_with_contract_nonce(
    identity_id: Identifier,
    contract_id: Identifier,
    fetched_nonce: u64,
) -> Sdk {
    let mut sdk = SdkBuilder::new_mock().build().expect("build mock sdk");

    sdk.mock()
        .expect_fetch::<IdentityContractNonceFetcher, _>(
            (identity_id, contract_id),
            Some(IdentityContractNonceFetcher(fetched_nonce)),
        )
        .await
        .expect("set nonce fetch expectation");

    sdk
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
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
            Err(Error::Protocol(ProtocolError::ConsensusError(ref consensus_error)))
                if matches!(**consensus_error, ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_)))
        ),
        "unexpected result: {:?}",
        result
    );
}
