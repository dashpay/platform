//! A token whose configuration sets `transferable: false` (protocol version 14): no
//! `TokenTransfer` moves it, and a document of its own contract may still charge it by burning.
use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
use dpp::data_contract::document_type::random_document::{
    DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0Setters;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use drive::util::test_helpers::setup_contract;

fn make_non_transferable(token_configuration: &mut TokenConfiguration) {
    token_configuration.set_transferable(false);
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
        Some(make_non_transferable),
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
        }] if *inner.token_id() == token_id && inner.action() == "transfer" && error.code() == 40726
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

    // The card game's "card" type burns 10 of its gold token (position 0) on create.
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
            make_non_transferable(
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
