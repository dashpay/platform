//! A token whose configuration sets `transferable: false` (protocol version 14): no
//! `TokenTransfer` moves it, and a document of its own contract may still charge it by burning.
use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::validation::state_transition::tests::make_token_non_transferable;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
use dpp::data_contract::document_type::accessors::{
    DocumentTypeV0MutGetters, DocumentTypeV1Setters,
};
use dpp::data_contract::document_type::random_document::{
    DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0Setters;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use drive::util::test_helpers::setup_contract;

/// Takes the card type's purchase cost out of a card game contract, in the parsed type and in
/// its schema: it pays the owner in gold, which a non-transferable gold forbids.
fn drop_card_purchase_cost(data_contract: &mut DataContract) {
    let card = data_contract
        .document_types_mut()
        .get_mut("card")
        .expect("expected a card document type");
    card.set_document_purchase_token_cost(None);
    card.schema_mut()
        .get_mut("tokenCost")
        .expect("expected to get token cost")
        .expect("expected token cost to be set")
        .as_map_mut()
        .expect("expected token cost to be a map")
        .retain(|(action, _)| action.as_text() != Some("purchase"));
}

#[tokio::test]
async fn should_refuse_a_transfer_of_a_non_transferable_token_in_the_mempool_and_in_a_block() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let mut rng = StdRng::seed_from_u64(49853);

    let platform_state = platform.state.load();

    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

    let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        identity.id(),
        Some(make_token_non_transferable),
        None,
        None,
        None,
        platform_version,
    );

    let token_transfer_transition = BatchTransition::new_token_transfer_transition(
        token_id,
        identity.id(),
        contract.id(),
        0,
        1337,
        recipient.id(),
        None,
        None,
        None,
        &key,
        2,
        0,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create documents batch transition");

    let token_transfer_serialized_transition = token_transfer_transition
        .serialize_to_bytes()
        .expect("expected documents batch serialized state transition");

    // The rule reads only the contract the action carries, so the mempool refuses it too.
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let check_tx_result = platform
        .check_tx(
            &token_transfer_serialized_transition,
            CheckTxLevel::FirstTimeCheck,
            &platform_ref,
            platform_version,
        )
        .expect("expected to be able to check tx");
    assert_matches!(
        check_tx_result.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::TokenNotTransferableError(_)
        )]
    );

    let transaction = platform.drive.grove.start_transaction();

    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[token_transfer_serialized_transition],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process state transition");

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [PaidConsensusError {
            error: error @ ConsensusError::StateError(StateError::TokenNotTransferableError(inner)),
            ..
        }] if *inner.token_id() == token_id && inner.action() == "a token transfer" && error.code() == 40726
    );

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");

    let sender_balance = platform
        .drive
        .fetch_identity_token_balance(
            token_id.to_buffer(),
            identity.id().to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to fetch token balance");
    assert_eq!(sender_balance, Some(100000));

    let recipient_balance = platform
        .drive
        .fetch_identity_token_balance(
            token_id.to_buffer(),
            recipient.id().to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to fetch token balance");
    assert_eq!(recipient_balance, None);
}

#[tokio::test]
async fn should_let_a_non_transferable_token_pay_its_own_contracts_document_by_burning_it() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let mut rng = StdRng::seed_from_u64(433);

    let platform_state = platform.state.load();

    let (contract_owner, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

    let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

    // The card game's "card" type burns 10 of its gold token (position 0) on create. Its purchase
    // cost pays the owner in gold, which registration refuses for a non-transferable token
    // (10280), so it is taken out to keep the contract one the chain could hold.
    let data_contract_id = DataContract::generate_data_contract_id_v0(contract_owner.id(), 1);
    let contract = setup_contract(
        &platform.drive,
        "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json",
        Some(data_contract_id.to_buffer()),
        Some(contract_owner.id().to_buffer()),
        Some(|data_contract: &mut DataContract| {
            data_contract.set_created_at_epoch(Some(0));
            data_contract.set_created_at(Some(0));
            data_contract.set_created_at_block_height(Some(0));
            make_token_non_transferable(
                data_contract
                    .token_configuration_mut(0)
                    .expect("expected the gold token"),
            );
            drop_card_purchase_cost(data_contract);
        }),
        None,
        Some(platform_version),
    );
    let gold_token_id: Identifier = calculate_token_id(data_contract_id.as_bytes(), 0).into();

    add_tokens_to_identity(&platform, gold_token_id, buyer.id(), 15);

    let card_document_type = contract
        .document_type_for_name("card")
        .expect("expected a card document type");

    let entropy = Bytes32::random_with_rng(&mut rng);

    let mut document = card_document_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            buyer.id(),
            entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random document");
    document
        .set_id_for_creation(card_document_type, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    document.set("attack", 4.into());
    document.set("defense", 7.into());

    let documents_batch_create_transition =
        BatchTransition::new_document_creation_transition_from_document(
            document,
            card_document_type,
            entropy.0,
            &key,
            2,
            0,
            Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                payment_token_contract_id: None,
                token_contract_position: 0,
                minimum_token_cost: None,
                maximum_token_cost: Some(10),
                gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            })),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create documents batch transition");

    let transaction = platform.drive.grove.start_transaction();

    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[documents_batch_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition")],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process state transition");

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");

    let buyer_balance = platform
        .drive
        .fetch_identity_token_balance(
            gold_token_id.to_buffer(),
            buyer.id().to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to fetch token balance");
    assert_eq!(buyer_balance, Some(5));

    let token_supply = platform
        .drive
        .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
        .expect("expected to fetch total supply");
    assert_eq!(token_supply, Some(5));
}

/// Registration refuses a cost that pays the owner in the contract's own non-transferable token
/// (10280), so the contract is inserted directly to show that a document payment refuses it
/// too, without reading anything.
#[tokio::test]
async fn should_refuse_paying_the_contract_owner_its_own_non_transferable_token_at_payment_time() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let mut rng = StdRng::seed_from_u64(433);

    let platform_state = platform.state.load();

    let (contract_owner, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

    let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

    // The card game's "card" type pays 10 of its gold token (position 0) to the owner on create.
    let data_contract_id = DataContract::generate_data_contract_id_v0(contract_owner.id(), 1);
    let contract = setup_contract(
        &platform.drive,
        "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
        Some(data_contract_id.to_buffer()),
        Some(contract_owner.id().to_buffer()),
        Some(|data_contract: &mut DataContract| {
            data_contract.set_created_at_epoch(Some(0));
            data_contract.set_created_at(Some(0));
            data_contract.set_created_at_block_height(Some(0));
            make_token_non_transferable(
                data_contract
                    .token_configuration_mut(0)
                    .expect("expected the gold token"),
            );
        }),
        None,
        Some(platform_version),
    );
    let gold_token_id: Identifier = calculate_token_id(data_contract_id.as_bytes(), 0).into();

    add_tokens_to_identity(&platform, gold_token_id, buyer.id(), 15);

    let card_document_type = contract
        .document_type_for_name("card")
        .expect("expected a card document type");

    let entropy = Bytes32::random_with_rng(&mut rng);

    let mut document = card_document_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            buyer.id(),
            entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random document");
    document
        .set_id_for_creation(card_document_type, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    document.set("attack", 4.into());
    document.set("defense", 7.into());

    let documents_batch_create_transition =
        BatchTransition::new_document_creation_transition_from_document(
            document,
            card_document_type,
            entropy.0,
            &key,
            2,
            0,
            Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                payment_token_contract_id: None,
                token_contract_position: 0,
                minimum_token_cost: None,
                maximum_token_cost: Some(10),
                gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            })),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create documents batch transition");

    let transaction = platform.drive.grove.start_transaction();

    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[documents_batch_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition")],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process state transition");

    assert_matches!(
        processing_result.execution_results().as_slice(),
        [PaidConsensusError {
            error: ConsensusError::StateError(StateError::TokenNotTransferableError(inner)),
            ..
        }] if *inner.token_id() == gold_token_id
            && inner.action() == "a document create token payment to the contract owner"
    );

    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");

    let buyer_balance = platform
        .drive
        .fetch_identity_token_balance(
            gold_token_id.to_buffer(),
            buyer.id().to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to fetch token balance");
    assert_eq!(buyer_balance, Some(15));
}

/// What an issuer hands a non-transferable token out with: a mint to a chosen identity, a
/// once-per-identity claim and a direct purchase each still credit the identity.
#[tokio::test]
async fn should_still_hand_a_non_transferable_token_out_by_mint_claim_and_purchase() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let mut rng = StdRng::seed_from_u64(12345);

    let (owner, owner_signer, owner_key) =
        setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let (minted_to, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (claimant, claimant_signer, claimant_key) =
        setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let (buyer, buyer_signer, buyer_key) =
        setup_identity(&mut platform, rng.gen(), dash_to_credits!(10.0));

    let owner_rules = || {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        })
    };
    let (contract, token_id) = create_token_contract_with_owner_identity(
        &mut platform,
        owner.id(),
        Some(|token_configuration: &mut TokenConfiguration| {
            make_token_non_transferable(token_configuration);
            token_configuration.set_manual_minting_rules(owner_rules());
            let distribution_rules = token_configuration.distribution_rules_mut();
            distribution_rules.set_minting_allow_choosing_destination(true);
            distribution_rules.set_change_direct_purchase_pricing_rules(owner_rules());
            distribution_rules.set_once_per_identity_distribution(Some(
                TokenOncePerIdentityDistribution::V0(TokenOncePerIdentityDistributionV0 {
                    amount: 100,
                }),
            ));
        }),
        None,
        None,
        None,
        platform_version,
    );

    let platform_state = platform.state.load();

    let mint = BatchTransition::new_token_mint_transition(
        token_id,
        owner.id(),
        contract.id(),
        0,
        50,
        Some(minted_to.id()),
        None,
        None,
        &owner_key,
        2,
        0,
        &owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create the mint transition");
    let claim = BatchTransition::new_token_claim_transition(
        token_id,
        claimant.id(),
        contract.id(),
        0,
        TokenDistributionType::OncePerIdentity,
        None,
        &claimant_key,
        2,
        0,
        &claimant_signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create the claim transition");
    let set_price = BatchTransition::new_token_change_direct_purchase_price_transition(
        token_id,
        owner.id(),
        contract.id(),
        0,
        Some(TokenPricingSchedule::SinglePrice(dash_to_credits!(1))),
        None,
        None,
        &owner_key,
        3,
        0,
        &owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create the price transition");
    let purchase = BatchTransition::new_token_direct_purchase_transition(
        token_id,
        buyer.id(),
        contract.id(),
        0,
        3,
        dash_to_credits!(3),
        &buyer_key,
        2,
        0,
        &buyer_signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create the purchase transition");

    for transition in [mint, claim, set_price, purchase] {
        let processing_result = process_test_state_transition(
            &mut platform,
            transition,
            &platform_state,
            platform_version,
        );
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
    }

    for (identity_id, expected) in [(minted_to.id(), 50), (claimant.id(), 100), (buyer.id(), 3)] {
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(balance, Some(expected));
    }
}
