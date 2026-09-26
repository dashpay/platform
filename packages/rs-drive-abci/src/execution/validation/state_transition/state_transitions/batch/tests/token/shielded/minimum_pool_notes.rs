//! A token shielded pool's outgoing notes threshold, `minimumPoolNotesForOutgoing`.
//!
//! The issuer sets it per token, at most `max_token_pool_notes_for_outgoing`, and changes it
//! only through `TokenConfigUpdate` under the token's own rules. While the pool holds fewer
//! notes, every path that moves tokens out of the pool to a visible destination is refused:
//! unshielding to an identity, with the identity's fee or with a fee from the credit pool,
//! burning from the pool, and paying a document's token cost from it. Transfers inside the
//! pool are never refused, since they are how the count grows.

use super::document_shielded_token_payment_tests as cards;
use super::token_pool_paid_transitions_tests::{
    credit_pool_balance, fund_credit_pool, fund_token_pool, spendable, wallet_address, Prover,
    CREDIT_NOTE, SHIELDED,
};
use super::token_shielded_pool_tests::{
    assert_tokens_conserved, build_shield_bundle, build_spend_bundle, identity_token_balance,
    insert_token_pool_anchor, nullifier_is_spent, platform_with_latest_version, pool_balance,
    pool_notes_count, process, shielded_token_contract, spend_keys, spendable_note, SHIELD_AMOUNT,
};
use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::validation::state_transition::tests::create_card_game_external_token_contract_with_owner_identity;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::{
    TokenConfigurationV1Getters, TokenConfigurationV1Setters,
};
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::config::v0::DataContractConfigSettersV0;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::shielded::builder::{
    build_token_shielded_transfer_with_shielded_fee_transition,
    build_token_unshield_with_shielded_fee_transition, ShieldedFeePayer, TokenPoolSpender,
};
use dpp::shielded::{
    document_token_payment_extra_sighash_data_v0, token_burn_from_pool_extra_sighash_data_v0,
    token_shielded_transfer_extra_sighash_data_v0, token_unshield_extra_sighash_data_v0,
};
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::v1::TokenPaymentInfoV1;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
use drive::util::test_helpers::setup_contract;
use platform_version::version::PlatformVersion;
use simple_signer::signer::SimpleSigner;

/// One shield puts two notes in an empty pool and one spend with its change two more, so a
/// threshold of 4 refuses the first outflow and admits it once one transfer happened inside
/// the pool. Each test asserts the counts it relies on rather than trusting this arithmetic.
const THRESHOLD: u64 = 4;

/// Gives the token a shielded pool whose outflows wait for `minimum_pool_notes` notes.
fn pooled_with_threshold(configuration: &mut TokenConfiguration, minimum_pool_notes: u64) {
    configuration.set_has_shielded_pool(true);
    let TokenConfiguration::V1(v1) = configuration else {
        panic!("a token with a pool has a V1 configuration");
    };
    v1.minimum_pool_notes_for_outgoing = Some(minimum_pool_notes);
}

fn contract_owner_rules() -> ChangeControlRules {
    ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
        admin_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_admin_action_takers_to_no_one_allowed: false,
        self_changing_admin_action_takers_allowed: false,
    })
}

fn max_threshold() -> u64 {
    PlatformVersion::latest()
        .system_limits
        .max_token_pool_notes_for_outgoing
}

/// The threshold of token 0 as the contract stored in Drive declares it.
fn stored_threshold(platform: &TempPlatform<MockCoreRPCLike>, contract_id: Identifier) -> u64 {
    platform
        .drive
        .fetch_contract(
            contract_id.to_buffer(),
            None,
            None,
            None,
            PlatformVersion::latest(),
        )
        .unwrap()
        .expect("fetch contract")
        .expect("contract exists")
        .contract
        .expected_token_configuration(0)
        .expect("token 0")
        .minimum_pool_notes_for_outgoing()
}

fn assert_refused_below(errors: &[ConsensusError], notes: u64, minimum_required: u64) {
    assert_matches!(
        errors,
        [ConsensusError::StateError(StateError::InsufficientPoolNotesError(error))]
            if error.current_count() == notes && error.minimum_required() == minimum_required
    );
}

fn assert_refused_below_threshold(errors: &[ConsensusError], notes: u64) {
    assert_refused_below(errors, notes, THRESHOLD);
}

/// The errors CheckTx answers with at mempool admission.
fn check_tx_errors(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: &StateTransition,
) -> Vec<ConsensusError> {
    let state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    platform
        .check_tx(
            &transition.serialize_to_bytes().expect("serialize"),
            CheckTxLevel::FirstTimeCheck,
            &platform_ref,
            PlatformVersion::latest(),
        )
        .expect("check tx")
        .errors
}

/// Shields `SHIELD_AMOUNT` of the owner's tokens into the pool.
#[allow(clippy::too_many_arguments)]
async fn shield(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    token_id: Identifier,
    owner: &Identity,
    key: &IdentityPublicKey,
    signer: &SimpleSigner,
    nonce: u64,
    seed: u64,
) {
    let shield = BatchTransition::new_token_shield_transition(
        token_id,
        owner.id(),
        contract.id(),
        0,
        SHIELD_AMOUNT,
        build_shield_bundle(
            SHIELD_AMOUNT,
            seed,
            TokenTransitionActionType::Shield,
            token_id,
            owner.id(),
        ),
        key,
        nonce,
        0,
        signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("token shield transition");
    let result = process(platform, &shield);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
}

/// A token pool written without a threshold, the way every pool was before the setting
/// existed, reads as 0 and lets its first holder leave: the default must not trap the first
/// depositors of a new pool.
#[tokio::test]
async fn should_read_a_pooled_token_without_a_threshold_as_zero_and_let_its_first_holder_leave() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9501);
    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        identity.id(),
        Some(|configuration: &mut TokenConfiguration| configuration.set_has_shielded_pool(true)),
        None,
        None,
        None,
        platform_version,
    );
    let TokenConfiguration::V1(v1) = contract.expected_token_configuration(0).expect("token 0")
    else {
        panic!("a token with a pool has a V1 configuration");
    };
    assert_eq!(v1.minimum_pool_notes_for_outgoing, None);
    assert_eq!(stored_threshold(&platform, contract.id()), 0);

    shield(
        &platform, &contract, token_id, &identity, &key, &signer, 2, 91,
    )
    .await;
    assert_eq!(pool_notes_count(&platform, token_id), 2);

    let token = token_id.to_buffer();
    let (note, anchor, merkle_path) = spendable_note(6_000, 1);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let extra = token_unshield_extra_sighash_data_v0(
        &token,
        &identity.id().to_buffer(),
        &recipient.id().to_buffer(),
        4_000,
    );
    let (bundle, _) = build_spend_bundle(note, merkle_path, anchor, 4_000, &extra, 92);
    let unshield = BatchTransition::new_token_unshield_transition(
        token_id,
        identity.id(),
        contract.id(),
        0,
        4_000,
        recipient.id(),
        bundle,
        &key,
        3,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("token unshield transition");
    let result = process(&platform, &unshield);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(
        identity_token_balance(&platform, token_id, recipient.id()),
        Some(4_000)
    );
    assert_tokens_conserved(&platform);
}

/// An unshield to an identity waits for the threshold, a transfer inside the pool does not,
/// and the very unshield that was refused goes through once that transfer lifted the count:
/// the threshold was the only reason it was refused.
#[tokio::test]
async fn should_refuse_an_unshield_below_the_threshold_but_never_a_transfer_inside_the_pool() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9502);
    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        identity.id(),
        Some(|configuration: &mut TokenConfiguration| {
            pooled_with_threshold(configuration, THRESHOLD)
        }),
        None,
        None,
        None,
        platform_version,
    );
    let token = token_id.to_buffer();
    shield(
        &platform, &contract, token_id, &identity, &key, &signer, 2, 93,
    )
    .await;
    let notes = pool_notes_count(&platform, token_id);
    assert!(notes < THRESHOLD);

    let unshield_amount = 4_000;
    let (note, anchor, merkle_path) = spendable_note(6_000, 1);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let extra = token_unshield_extra_sighash_data_v0(
        &token,
        &identity.id().to_buffer(),
        &recipient.id().to_buffer(),
        unshield_amount,
    );
    let (unshield_bundle, _) =
        build_spend_bundle(note, merkle_path, anchor, unshield_amount, &extra, 94);
    let unshield_with_nonce = |nonce| {
        BatchTransition::new_token_unshield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            unshield_amount,
            recipient.id(),
            unshield_bundle.clone(),
            &key,
            nonce,
            0,
            &signer,
            platform_version,
            None,
        )
    };

    let unshield = unshield_with_nonce(3).await.expect("token unshield");
    let result = process(&platform, &unshield);
    let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected a paid refusal, got {:?}",
            result.execution_results()
        );
    };
    assert_refused_below_threshold(std::slice::from_ref(error), notes);
    assert_eq!(pool_balance(&platform, token_id), SHIELD_AMOUNT);
    assert_eq!(
        identity_token_balance(&platform, token_id, recipient.id()),
        None
    );
    for action in &unshield_bundle.actions {
        assert!(!nullifier_is_spent(&platform, token_id, &action.nullifier));
    }

    let (note, anchor, merkle_path) = spendable_note(3_000, 2);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let extra = token_shielded_transfer_extra_sighash_data_v0(&token, &identity.id().to_buffer());
    let (transfer_bundle, _) = build_spend_bundle(note, merkle_path, anchor, 0, &extra, 95);
    let transfer = BatchTransition::new_token_shielded_transfer_transition(
        token_id,
        identity.id(),
        contract.id(),
        0,
        transfer_bundle,
        &key,
        4,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("token shielded transfer");
    let result = process(&platform, &transfer);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(pool_notes_count(&platform, token_id), THRESHOLD);

    let unshield = unshield_with_nonce(5).await.expect("token unshield");
    let result = process(&platform, &unshield);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(
        identity_token_balance(&platform, token_id, recipient.id()),
        Some(unshield_amount)
    );
    assert_eq!(
        pool_balance(&platform, token_id),
        SHIELD_AMOUNT - unshield_amount
    );
    assert_tokens_conserved(&platform);
}

#[tokio::test]
async fn should_refuse_a_burn_from_the_pool_below_the_threshold() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9503);
    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        identity.id(),
        Some(|configuration: &mut TokenConfiguration| {
            pooled_with_threshold(configuration, THRESHOLD)
        }),
        None,
        None,
        None,
        platform_version,
    );
    shield(
        &platform, &contract, token_id, &identity, &key, &signer, 2, 96,
    )
    .await;
    let notes = pool_notes_count(&platform, token_id);
    assert!(notes < THRESHOLD);

    let burn_amount = 4_000;
    let (note, anchor, merkle_path) = spendable_note(6_000, 3);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let extra = token_burn_from_pool_extra_sighash_data_v0(
        &token_id.to_buffer(),
        &identity.id().to_buffer(),
        burn_amount,
    );
    let (burn_bundle, _) = build_spend_bundle(note, merkle_path, anchor, burn_amount, &extra, 97);
    let burn = BatchTransition::new_token_burn_from_pool_transition(
        token_id,
        identity.id(),
        contract.id(),
        0,
        burn_amount,
        burn_bundle.clone(),
        None,
        None,
        &key,
        3,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("token burn from pool transition");
    let result = process(&platform, &burn);
    let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected a paid refusal, got {:?}",
            result.execution_results()
        );
    };
    assert_refused_below_threshold(std::slice::from_ref(error), notes);
    assert_eq!(pool_balance(&platform, token_id), SHIELD_AMOUNT);
    for action in &burn_bundle.actions {
        assert!(!nullifier_is_spent(&platform, token_id, &action.nullifier));
    }
    assert_tokens_conserved(&platform);
}

/// The card game contract with a burned creation cost in its own token 0, which has a pool
/// with the test threshold.
fn card_game_contract_with_threshold(
    platform: &TempPlatform<MockCoreRPCLike>,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> (DataContract, Identifier) {
    let data_contract_id = DataContract::generate_data_contract_id_v0(owner_id, 1);
    let contract = setup_contract(
        &platform.drive,
        "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json",
        Some(data_contract_id.to_buffer()),
        Some(owner_id.to_buffer()),
        Some(|data_contract: &mut DataContract| {
            data_contract.set_created_at_epoch(Some(0));
            data_contract.set_created_at(Some(0));
            data_contract.set_created_at_block_height(Some(0));
            let configuration = data_contract
                .tokens_mut()
                .and_then(|tokens| tokens.get_mut(&0))
                .expect("token 0");
            pooled_with_threshold(configuration, THRESHOLD);
        }),
        None,
        Some(platform_version),
    );
    (
        contract,
        calculate_token_id(data_contract_id.as_bytes(), 0).into(),
    )
}

#[tokio::test]
async fn should_refuse_a_document_token_cost_paid_from_the_pool_below_the_threshold() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9504);
    let (contract_owner, _, _) = setup_identity(&mut platform, 961, dash_to_credits!(0.1));
    let (buyer, signer, key) = setup_identity(&mut platform, 237, dash_to_credits!(0.5));
    let (contract, token_id) =
        card_game_contract_with_threshold(&platform, contract_owner.id(), platform_version);
    cards::fund_and_shield(
        &mut platform,
        &contract,
        token_id,
        &buyer,
        &key,
        &signer,
        98,
        platform_version,
    )
    .await;
    let notes = pool_notes_count(&platform, token_id);
    assert!(notes < THRESHOLD);

    let card_document_type = contract
        .document_type_for_name("card")
        .expect("card document type");
    let (document, entropy) = cards::random_card(
        &mut rng,
        card_document_type,
        buyer.id(),
        2,
        platform_version,
    );
    let (note, anchor, merkle_path) = spendable_note(cards::SHIELDED, 7);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let extra = document_token_payment_extra_sighash_data_v0(
        &token_id.to_buffer(),
        &buyer.id().to_buffer(),
        &contract.id().to_buffer(),
        &document.id().to_buffer(),
        cards::CARD_COST,
    );
    let (bundle, _) = build_spend_bundle(note, merkle_path, anchor, cards::CARD_COST, &extra, 99);
    let payment = cards::shielded_payment(bundle, cards::CARD_COST);
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        card_document_type,
        entropy.0,
        &key,
        2,
        0,
        Some(cards::payment_info(payment.clone())),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("document create transition");

    let result = process(&platform, &transition);
    let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected a paid refusal, got {:?}",
            result.execution_results()
        );
    };
    assert_refused_below_threshold(std::slice::from_ref(error), notes);
    assert_eq!(pool_balance(&platform, token_id), cards::SHIELDED);
    assert!(!nullifier_is_spent(
        &platform,
        token_id,
        &payment.actions[0].nullifier
    ));
    assert_tokens_conserved(&platform);
}

/// The document's cost is in a token another contract declares, so the threshold is read from
/// that contract, which the document's own does not carry; once the pool holds enough notes
/// the same cost is paid from it.
#[tokio::test]
async fn should_read_the_threshold_of_a_token_declared_by_another_contract() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9505);
    let (holder, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (card_owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
    let (token_contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        holder.id(),
        Some(|configuration: &mut TokenConfiguration| {
            pooled_with_threshold(configuration, THRESHOLD)
        }),
        None,
        None,
        None,
        platform_version,
    );
    let card_contract = create_card_game_external_token_contract_with_owner_identity(
        &mut platform,
        token_contract.id(),
        0,
        cards::CARD_COST,
        GasFeesPaidBy::DocumentOwner,
        card_owner.id(),
        platform_version,
    );
    shield(
        &platform,
        &token_contract,
        token_id,
        &holder,
        &key,
        &signer,
        2,
        100,
    )
    .await;
    let notes = pool_notes_count(&platform, token_id);
    assert!(notes < THRESHOLD);

    let card_document_type = card_contract
        .document_type_for_name("card")
        .expect("card document type");
    let (note, anchor, merkle_path) = spendable_note(6_000, 8);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    // A card paid from the pool, created under `nonce`.
    let mut rng_for_cards = StdRng::seed_from_u64(9515);
    let mut paid_card = |nonce: u64, seed: u64| {
        let (document, entropy) = cards::random_card(
            &mut rng_for_cards,
            card_document_type,
            holder.id(),
            nonce,
            platform_version,
        );
        let extra = document_token_payment_extra_sighash_data_v0(
            &token_id.to_buffer(),
            &holder.id().to_buffer(),
            &card_contract.id().to_buffer(),
            &document.id().to_buffer(),
            cards::CARD_COST,
        );
        let (bundle, _) = build_spend_bundle(
            note,
            merkle_path.clone(),
            anchor,
            cards::CARD_COST,
            &extra,
            seed,
        );
        let payment_info = TokenPaymentInfo::V1(TokenPaymentInfoV1 {
            payment_token_contract_id: Some(token_contract.id()),
            token_contract_position: 0,
            minimum_token_cost: None,
            maximum_token_cost: Some(cards::CARD_COST),
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            shielded_payment: Box::new(cards::shielded_payment(bundle, cards::CARD_COST)),
        });
        (document, entropy, payment_info)
    };

    let (document, entropy, payment_info) = paid_card(1, 101);
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        card_document_type,
        entropy.0,
        &key,
        1,
        0,
        Some(payment_info),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("document create transition");
    let result = process(&platform, &transition);
    let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected a paid refusal, got {:?}",
            result.execution_results()
        );
    };
    assert_refused_below_threshold(std::slice::from_ref(error), notes);
    assert_eq!(pool_balance(&platform, token_id), SHIELD_AMOUNT);

    let (transfer_note, transfer_anchor, transfer_path) = spendable_note(3_000, 10);
    insert_token_pool_anchor(&platform, token_id, &transfer_anchor);
    let extra = token_shielded_transfer_extra_sighash_data_v0(
        &token_id.to_buffer(),
        &holder.id().to_buffer(),
    );
    let (transfer_bundle, _) = build_spend_bundle(
        transfer_note,
        transfer_path,
        transfer_anchor,
        0,
        &extra,
        105,
    );
    let transfer = BatchTransition::new_token_shielded_transfer_transition(
        token_id,
        holder.id(),
        token_contract.id(),
        0,
        transfer_bundle,
        &key,
        3,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("token shielded transfer");
    let result = process(&platform, &transfer);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(pool_notes_count(&platform, token_id), THRESHOLD);

    let (document, entropy, payment_info) = paid_card(2, 106);
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        card_document_type,
        entropy.0,
        &key,
        2,
        0,
        Some(payment_info),
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("document create transition");
    let result = process(&platform, &transition);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(
        pool_balance(&platform, token_id),
        SHIELD_AMOUNT - cards::CARD_COST
    );
    assert_eq!(
        identity_token_balance(&platform, token_id, card_owner.id()),
        Some(cards::CARD_COST)
    );
    assert_tokens_conserved(&platform);
}

/// `TokenUnshieldWithShieldedFee` moves tokens out of the pool to a visible identity like
/// `TokenUnshield` does; only its fee comes from the credit pool. It is refused below the
/// threshold at mempool admission and in the block, while `TokenShieldedTransferWithShieldedFee`,
/// which moves nothing out, is not; and the refused transition itself goes through once that
/// transfer lifted the count.
#[tokio::test]
async fn should_refuse_an_unshield_with_a_shielded_fee_below_the_threshold() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9506);
    let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        owner.id(),
        Some(|configuration: &mut TokenConfiguration| {
            pooled_with_threshold(configuration, THRESHOLD)
        }),
        None,
        None,
        None,
        platform_version,
    );
    let (token_note, token_anchor, token_path) = fund_token_pool(
        &mut platform,
        &contract,
        token_id,
        &owner,
        &key,
        &signer,
        102,
        21,
        platform_version,
    )
    .await;
    let notes = pool_notes_count(&platform, token_id);
    assert!(notes < THRESHOLD);

    let (credit_note, credit_anchor, credit_path) = fund_credit_pool(&platform, 22);
    let (fvk, ask, _) = spend_keys();
    let address = wallet_address();
    let (unshield, fee) = build_token_unshield_with_shielded_fee_transition(
        token_id,
        contract.id(),
        0,
        TokenPoolSpender {
            spends: vec![spendable(token_note, token_path)],
            change_address: &address,
            fvk: &fvk,
            ask: &ask,
            anchor: token_anchor,
        },
        recipient.id(),
        10,
        [0u8; 36],
        ShieldedFeePayer {
            spends: vec![spendable(credit_note, credit_path)],
            change_address: &address,
            fvk: &fvk,
            ask: &ask,
            anchor: credit_anchor,
        },
        &Prover,
        platform_version,
    )
    .expect("build transition");

    assert_refused_below_threshold(&check_tx_errors(&platform, &unshield), notes);
    let result = process(&platform, &unshield);
    let [StateTransitionExecutionResult::UnpaidConsensusError(error)] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected an unpaid refusal, got {:?}",
            result.execution_results()
        );
    };
    assert_refused_below_threshold(std::slice::from_ref(error), notes);
    assert_eq!(pool_balance(&platform, token_id), SHIELDED);
    assert_eq!(
        identity_token_balance(&platform, token_id, recipient.id()),
        None
    );
    assert_eq!(credit_pool_balance(&platform), CREDIT_NOTE);

    let (transfer_note, transfer_anchor, transfer_path) = spendable_note(SHIELDED, 23);
    insert_token_pool_anchor(&platform, token_id, &transfer_anchor);
    let (credit_note, credit_anchor, credit_path) = fund_credit_pool(&platform, 24);
    let (transfer, transfer_fee) = build_token_shielded_transfer_with_shielded_fee_transition(
        token_id,
        contract.id(),
        0,
        TokenPoolSpender {
            spends: vec![spendable(transfer_note, transfer_path)],
            change_address: &address,
            fvk: &fvk,
            ask: &ask,
            anchor: transfer_anchor,
        },
        &address,
        6,
        [0u8; 36],
        ShieldedFeePayer {
            spends: vec![spendable(credit_note, credit_path)],
            change_address: &address,
            fvk: &fvk,
            ask: &ask,
            anchor: credit_anchor,
        },
        &Prover,
        platform_version,
    )
    .expect("build transition");
    let result = process(&platform, &transfer);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(pool_notes_count(&platform, token_id), THRESHOLD);

    let result = process(&platform, &unshield);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(pool_balance(&platform, token_id), SHIELDED - 10);
    assert_eq!(
        identity_token_balance(&platform, token_id, recipient.id()),
        Some(10)
    );
    assert_eq!(
        credit_pool_balance(&platform),
        CREDIT_NOTE - transfer_fee - fee
    );
    assert_tokens_conserved(&platform);
}

fn assert_refused_as_out_of_bounds(result: &[StateTransitionExecutionResult]) {
    assert_matches!(
        result,
        [StateTransitionExecutionResult::UnpaidConsensusError(
            error @ ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::KeyWrongBounds(_)
            ))
        )] if error.code() == 10241
    );
}

/// An issuer could otherwise set a threshold no pool ever reaches and strand every shielded
/// balance, irreversibly on a readonly contract.
#[tokio::test]
async fn should_refuse_a_threshold_above_the_limit_at_contract_create() {
    let platform_version = PlatformVersion::latest();
    for (minimum_pool_notes, accepted) in [(max_threshold(), true), (max_threshold() + 1, false)] {
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9507);
        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let mut contract = shielded_token_contract(identity.id(), platform_version);
        let contract_id = contract.id();
        pooled_with_threshold(
            contract.token_configuration_mut(0).expect("token 0"),
            minimum_pool_notes,
        );
        let create = DataContractCreateTransition::new_from_data_contract(
            contract,
            1,
            &identity.clone().into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("contract create transition");

        let result = process(&platform, &create);
        if accepted {
            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            assert_eq!(stored_threshold(&platform, contract_id), minimum_pool_notes);
        } else {
            assert_refused_as_out_of_bounds(result.execution_results());
        }
    }
}

#[tokio::test]
async fn should_refuse_a_threshold_above_the_limit_on_a_token_a_contract_update_adds() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let (identity, signer, key) = setup_identity(&mut platform, 9508, dash_to_credits!(1.0));
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    contract.set_owner_id(identity.id());
    contract.config_mut().set_readonly(false);
    platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("store original contract");

    let mut configuration = shielded_token_contract(identity.id(), platform_version)
        .expected_token_configuration(0)
        .expect("token 0")
        .clone();
    pooled_with_threshold(&mut configuration, max_threshold() + 1);
    contract.add_token(0, configuration);
    contract.increment_version();
    let update = DataContractUpdateTransition::new_from_data_contract(
        contract,
        &identity.into_partial_identity_info(),
        key.id(),
        1,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("contract update transition");

    let result = process(&platform, &update);
    assert_refused_as_out_of_bounds(result.execution_results());
}

/// A `TokenConfigUpdate` of token 0 applying `item`, signed by `identity`.
#[allow(clippy::too_many_arguments)]
async fn token_config_update(
    token_id: Identifier,
    contract_id: Identifier,
    identity: &Identity,
    key: &IdentityPublicKey,
    signer: &SimpleSigner,
    item: TokenConfigurationChangeItem,
    nonce: u64,
    platform_version: &PlatformVersion,
) -> StateTransition {
    BatchTransition::new_token_config_update_transition(
        token_id,
        identity.id(),
        contract_id,
        0,
        item,
        None,
        None,
        key,
        nonce,
        0,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("token config update transition")
}

/// The threshold changes the way any other token parameter does: through `TokenConfigUpdate`,
/// bounded by the same limit and authorized by its own rules, and the outflow checks read the
/// value it set. A contract update that tries to change it directly is still refused, as for
/// every other field of an existing token.
#[tokio::test]
async fn should_change_the_threshold_through_token_config_update_and_never_by_contract_update() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9509);
    let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (stranger, stranger_signer, stranger_key) =
        setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        owner.id(),
        Some(|configuration: &mut TokenConfiguration| {
            configuration.set_has_shielded_pool(true);
            let TokenConfiguration::V1(v1) = configuration else {
                panic!("a token with a pool has a V1 configuration");
            };
            v1.minimum_pool_notes_for_outgoing_change_rules = contract_owner_rules();
        }),
        None,
        None,
        None,
        platform_version,
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 0);
    let set_threshold = |minimum_pool_notes| {
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(minimum_pool_notes)
    };

    let over_the_limit = token_config_update(
        token_id,
        contract.id(),
        &owner,
        &key,
        &signer,
        set_threshold(max_threshold() + 1),
        2,
        platform_version,
    )
    .await;
    let result = process(&platform, &over_the_limit);
    assert_refused_as_out_of_bounds(result.execution_results());

    let by_a_stranger = token_config_update(
        token_id,
        contract.id(),
        &stranger,
        &stranger_key,
        &stranger_signer,
        set_threshold(5),
        2,
        platform_version,
    )
    .await;
    let result = process(&platform, &by_a_stranger);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
            ..
        }]
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 0);

    let raise = token_config_update(
        token_id,
        contract.id(),
        &owner,
        &key,
        &signer,
        set_threshold(5),
        3,
        platform_version,
    )
    .await;
    let result = process(&platform, &raise);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 5);

    // The raised threshold is the one an outflow now meets.
    shield(
        &platform, &contract, token_id, &owner, &key, &signer, 4, 103,
    )
    .await;
    let notes = pool_notes_count(&platform, token_id);
    assert!(notes < 5);
    let token = token_id.to_buffer();
    let (note, anchor, merkle_path) = spendable_note(6_000, 9);
    insert_token_pool_anchor(&platform, token_id, &anchor);
    let extra = token_unshield_extra_sighash_data_v0(
        &token,
        &owner.id().to_buffer(),
        &recipient.id().to_buffer(),
        4_000,
    );
    let (unshield_bundle, _) = build_spend_bundle(note, merkle_path, anchor, 4_000, &extra, 104);
    let unshield_with_nonce = |nonce| {
        BatchTransition::new_token_unshield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            4_000,
            recipient.id(),
            unshield_bundle.clone(),
            &key,
            nonce,
            0,
            &signer,
            platform_version,
            None,
        )
    };
    let unshield = unshield_with_nonce(5).await.expect("token unshield");
    let result = process(&platform, &unshield);
    let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
        result.execution_results().as_slice()
    else {
        panic!(
            "expected a paid refusal, got {:?}",
            result.execution_results()
        );
    };
    assert_refused_below(std::slice::from_ref(error), notes, 5);

    // Lowering it back to 0 releases the pool.
    let lower = token_config_update(
        token_id,
        contract.id(),
        &owner,
        &key,
        &signer,
        set_threshold(0),
        6,
        platform_version,
    )
    .await;
    let result = process(&platform, &lower);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 0);
    let unshield = unshield_with_nonce(7).await.expect("token unshield");
    let result = process(&platform, &unshield);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(
        identity_token_balance(&platform, token_id, recipient.id()),
        Some(4_000)
    );

    let mut updated = platform
        .drive
        .fetch_contract(
            contract.id().to_buffer(),
            None,
            None,
            None,
            platform_version,
        )
        .unwrap()
        .expect("fetch contract")
        .expect("contract exists")
        .contract
        .clone();
    pooled_with_threshold(updated.token_configuration_mut(0).expect("token 0"), 7);
    updated.increment_version();
    let direct = DataContractUpdateTransition::new_from_data_contract(
        updated,
        &owner.into_partial_identity_info(),
        key.id(),
        8,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("contract update transition");
    let result = process(&platform, &direct);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(
                StateError::DataContractUpdateActionNotAllowedError(error)
            ),
            ..
        }] if error.action() == "update token at position 0"
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 0);
}

/// Control of the threshold moves like control of any other token parameter: its admin hands
/// it to an identity, which has to exist, and that identity then sets the threshold in place
/// of the one that handed it over.
#[tokio::test]
async fn should_hand_control_of_the_threshold_only_to_an_identity_that_exists() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9511);
    let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (heir, heir_signer, heir_key) =
        setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        owner.id(),
        Some(|configuration: &mut TokenConfiguration| {
            configuration.set_has_shielded_pool(true);
            let TokenConfiguration::V1(v1) = configuration else {
                panic!("a token with a pool has a V1 configuration");
            };
            v1.minimum_pool_notes_for_outgoing_change_rules =
                ChangeControlRules::V0(ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                    admin_action_takers: AuthorizedActionTakers::ContractOwner,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                });
        }),
        None,
        None,
        None,
        platform_version,
    );
    let hand_control_to = |identity_id| {
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(
            AuthorizedActionTakers::Identity(identity_id),
        )
    };

    let missing = Identifier::from([0xAB; 32]);
    let to_nobody = token_config_update(
        token_id,
        contract.id(),
        &owner,
        &key,
        &signer,
        hand_control_to(missing),
        2,
        platform_version,
    )
    .await;
    let result = process(&platform, &to_nobody);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(
                StateError::NewAuthorizedActionTakerIdentityDoesNotExistError(_)
            ),
            ..
        }]
    );

    let to_the_heir = token_config_update(
        token_id,
        contract.id(),
        &owner,
        &key,
        &signer,
        hand_control_to(heir.id()),
        3,
        platform_version,
    )
    .await;
    let result = process(&platform, &to_the_heir);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );

    let by_the_owner = token_config_update(
        token_id,
        contract.id(),
        &owner,
        &key,
        &signer,
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(5),
        4,
        platform_version,
    )
    .await;
    let result = process(&platform, &by_the_owner);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
            ..
        }]
    );

    let by_the_heir = token_config_update(
        token_id,
        contract.id(),
        &heir,
        &heir_key,
        &heir_signer,
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(5),
        2,
        platform_version,
    )
    .await;
    let result = process(&platform, &by_the_heir);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 5);
}

/// A token without a pool has a V0 configuration, which has no threshold to change. Its
/// threshold items are refused as unauthorized, never accepted and applied to nothing.
#[tokio::test]
async fn should_refuse_the_threshold_items_for_a_token_without_a_pool() {
    let platform_version = PlatformVersion::latest();
    let mut platform = platform_with_latest_version();
    let mut rng = StdRng::seed_from_u64(9512);
    let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        owner.id(),
        None::<fn(&mut TokenConfiguration)>,
        None,
        None,
        None,
        platform_version,
    );

    for (item, nonce) in [
        (
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(5),
            2,
        ),
        (
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            3,
        ),
        (
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(
                AuthorizedActionTakers::ContractOwner,
            ),
            4,
        ),
    ] {
        let transition = token_config_update(
            token_id,
            contract.id(),
            &owner,
            &key,
            &signer,
            item,
            nonce,
            platform_version,
        )
        .await;
        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                ..
            }]
        );
    }

    let stored = platform
        .drive
        .fetch_contract(
            contract.id().to_buffer(),
            None,
            None,
            None,
            platform_version,
        )
        .unwrap()
        .expect("fetch contract")
        .expect("contract exists");
    assert_matches!(
        stored.contract.expected_token_configuration(0),
        Ok(TokenConfiguration::V0(_))
    );
    assert_eq!(stored_threshold(&platform, contract.id()), 0);
}

/// Before token shielded pools activate, no token has a threshold and software older than the
/// setting cannot decode its items, so they are refused unpaid like the pool transitions. Any
/// other configuration item still passes the gate.
#[tokio::test]
async fn should_refuse_the_threshold_items_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
        .expect("previous protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let mut rng = StdRng::seed_from_u64(9510);
    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        identity.id(),
        Some(|configuration: &mut TokenConfiguration| {
            configuration.set_max_supply_change_rules(contract_owner_rules())
        }),
        None,
        None,
        None,
        platform_version,
    );
    let process_at_version_13 = |transition: StateTransition| {
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[transition.serialize_to_bytes().expect("serialize")],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");
        result
    };

    for item in [
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(1),
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(
            AuthorizedActionTakers::ContractOwner,
        ),
        TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(
            AuthorizedActionTakers::ContractOwner,
        ),
    ] {
        let transition = token_config_update(
            token_id,
            contract.id(),
            &identity,
            &key,
            &signer,
            item,
            2,
            platform_version,
        )
        .await;
        let result = process_at_version_13(transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(_))
            )]
        );
    }

    let max_supply = token_config_update(
        token_id,
        contract.id(),
        &identity,
        &key,
        &signer,
        TokenConfigurationChangeItem::MaxSupply(Some(1_000_000)),
        2,
        platform_version,
    )
    .await;
    let result = process_at_version_13(max_supply);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
}
