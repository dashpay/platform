//! End-to-end coverage for random token amounts (protocol version 15): a token transfer or
//! mint naming a random minimum amount reserves the most it moves in the token marketplace
//! escrow when the block executes it, and `Drive::settle_random_token_amounts` pays out the
//! amount drawn from the block's random beacon once the block's transitions are accepted. The
//! token conservation check balances throughout. A client cannot know the amount, so it
//! reads it from the proof of its own transition.

use super::*;

mod random_amount_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::balances::credits::TokenAmount;
    use dpp::prelude::DataContract;
    use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
    use dpp::state_transition::batch_transition::{TokenMintTransition, TokenTransferTransition};
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use drive::drive::tokens::settle_random_token_amounts::{
        RandomTokenRequest, RandomTokenRequestKind,
    };
    use drive::drive::Drive;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const BEACON: [u8; 96] = [7; 96];
    const BASE_SUPPLY: TokenAmount = 100_000;
    const MINIMUM: TokenAmount = 100;
    const MAXIMUM: TokenAmount = 1_000;
    const NONCE: u64 = 2;

    fn random_options() -> Option<StateTransitionCreationOptions> {
        Some(StateTransitionCreationOptions {
            random_minimum_amount: Some(MINIMUM),
            ..Default::default()
        })
    }

    /// The token transition of the single-transition batch `transition`.
    fn token_transition(transition: &StateTransition) -> &TokenTransition {
        let StateTransition::Batch(BatchTransition::V1(batch)) = transition else {
            panic!("expected a version 1 batch");
        };
        let [BatchedTransition::Token(token_transition)] = batch.transitions.as_slice() else {
            panic!("expected a single token transition");
        };
        token_transition
    }

    fn process(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
    ) -> StateTransitionExecutionResult {
        let platform_version = PlatformVersion::latest();
        let platform_state = platform.state.load();
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the transition to serialize");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the transaction");
        processing_result.into_execution_results().remove(0)
    }

    fn balance(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        identity_id: Identifier,
    ) -> Option<TokenAmount> {
        platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch a token balance")
    }

    fn supply(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
    ) -> Option<TokenAmount> {
        platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, PlatformVersion::latest())
            .expect("expected to fetch the total supply")
    }

    /// The identity balances, the shielded pools and the token marketplace escrow, which holds
    /// the reservations, add up to the total supplies.
    fn assert_tokens_balanced(platform: &TempPlatform<MockCoreRPCLike>) {
        let total = platform
            .drive
            .calculate_total_tokens_balance(None, PlatformVersion::latest())
            .expect("expected to sum the token balances");
        assert!(
            total.ok().expect("expected the sums to compare"),
            "tokens are not balanced: {total}"
        );
    }

    /// Proves the committed state `transition` wrote and verifies the proof as the client that
    /// sent it does, holding the token's contract: it cannot know the drawn amount.
    fn verify_proof(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
        contract: &DataContract,
    ) -> StateTransitionProofResult {
        let platform_version = PlatformVersion::latest();
        let proof = platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the state transition")
            .into_data()
            .expect("expected proof bytes");
        let known_contracts: BTreeMap<Identifier, DataContract> =
            BTreeMap::from([(contract.id(), contract.clone())]);
        let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            &proof,
            &|id| Ok(known_contracts.get(id).cloned().map(Arc::new)),
            platform_version,
        )
        .expect("expected the proof to verify");
        outcome.into_result()
    }

    /// The amount of the history document a proof shows.
    fn proved_amount(result: StateTransitionProofResult) -> TokenAmount {
        let StateTransitionProofResult::VerifiedTokenActionWithDocument(document) = result else {
            panic!("expected the proof to show the history document, got {result:?}");
        };
        document
            .properties()
            .get_integer("amount")
            .expect("expected the history document to hold an amount")
    }

    fn settle(platform: &TempPlatform<MockCoreRPCLike>, request: RandomTokenRequest) {
        let settled = platform
            .drive
            .settle_random_token_amounts(
                &BEACON,
                &[request],
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected the settlement to run");
        assert_eq!(settled, 1);
    }

    #[tokio::test]
    async fn should_reserve_a_random_transfer_then_pay_out_the_drawn_amount() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(49855);
        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            MAXIMUM,
            recipient.id(),
            None,
            None,
            None,
            &key,
            NONCE,
            0,
            &signer,
            platform_version,
            random_options(),
        )
        .await
        .expect("expected to create the transfer");
        assert!(matches!(
            token_transition(&transition),
            TokenTransition::Transfer(TokenTransferTransition::V1(_))
        ));

        let result = process(&platform, &transition);
        assert_matches!(
            &result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
        // The block moves the most the transfer moves into a reservation, and writes the
        // recipient's balance at zero, where it waits for the draw
        assert_eq!(
            balance(&platform, token_id, identity.id()),
            Some(BASE_SUPPLY - MAXIMUM)
        );
        assert_eq!(balance(&platform, token_id, recipient.id()), Some(0));
        assert_tokens_balanced(&platform);
        // The history holds the range's minimum until the draw
        assert_eq!(
            proved_amount(verify_proof(&platform, &transition, &contract)),
            MINIMUM
        );

        settle(
            &platform,
            RandomTokenRequest {
                token_id,
                owner_id: identity.id(),
                identity_contract_nonce: NONCE,
                minimum: MINIMUM,
                maximum: MAXIMUM,
                kind: RandomTokenRequestKind::Transfer {
                    sender_id: identity.id(),
                    recipient_id: recipient.id(),
                },
            },
        );

        let drawn = balance(&platform, token_id, recipient.id()).expect("the recipient is paid");
        assert!((MINIMUM..=MAXIMUM).contains(&drawn));
        assert_eq!(
            balance(&platform, token_id, identity.id()),
            Some(BASE_SUPPLY - drawn)
        );
        assert_eq!(supply(&platform, token_id), Some(BASE_SUPPLY));
        assert_tokens_balanced(&platform);
        // The client that sent the transfer reads the drawn amount from the proof of its own
        // transition
        assert_eq!(
            proved_amount(verify_proof(&platform, &transition, &contract)),
            drawn
        );
    }

    #[tokio::test]
    async fn should_reserve_a_random_mint_then_pay_out_the_drawn_amount() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(49856);
        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            MAXIMUM,
            Some(recipient.id()),
            None,
            None,
            &key,
            NONCE,
            0,
            &signer,
            platform_version,
            random_options(),
        )
        .await
        .expect("expected to create the mint");
        assert!(matches!(
            token_transition(&transition),
            TokenTransition::Mint(TokenMintTransition::V1(_))
        ));

        let result = process(&platform, &transition);
        assert_matches!(
            &result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
        // The block adds the most the mint issues to the supply and holds it in a reservation,
        // and writes the recipient's balance at zero, where it waits for the draw
        assert_eq!(supply(&platform, token_id), Some(BASE_SUPPLY + MAXIMUM));
        assert_eq!(balance(&platform, token_id, recipient.id()), Some(0));
        assert_tokens_balanced(&platform);

        settle(
            &platform,
            RandomTokenRequest {
                token_id,
                owner_id: identity.id(),
                identity_contract_nonce: NONCE,
                minimum: MINIMUM,
                maximum: MAXIMUM,
                kind: RandomTokenRequestKind::Mint {
                    recipient_id: recipient.id(),
                },
            },
        );

        let drawn = balance(&platform, token_id, recipient.id()).expect("the recipient is paid");
        assert!((MINIMUM..=MAXIMUM).contains(&drawn));
        assert_eq!(supply(&platform, token_id), Some(BASE_SUPPLY + drawn));
        assert_eq!(
            balance(&platform, token_id, identity.id()),
            Some(BASE_SUPPLY)
        );
        assert_tokens_balanced(&platform);
        assert_eq!(
            proved_amount(verify_proof(&platform, &transition, &contract)),
            drawn
        );
    }

    #[tokio::test]
    async fn should_refuse_a_random_mint_without_a_recipient() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(49857);
        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_new_tokens_destination_identity(Some(identity.id()));
            }),
            None,
            None,
            None,
            platform_version,
        );

        let transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            MAXIMUM,
            None,
            None,
            None,
            &key,
            NONCE,
            0,
            &signer,
            platform_version,
            random_options(),
        )
        .await
        .expect("expected to create the mint");

        let error = match process(&platform, &transition) {
            StateTransitionExecutionResult::UnpaidConsensusError(error) => error,
            StateTransitionExecutionResult::PaidConsensusError { error, .. } => error,
            other => panic!("expected the mint to be refused, got {other:?}"),
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::RandomTokenMintNotAllowedError(_))
        );
        assert_eq!(supply(&platform, token_id), Some(BASE_SUPPLY));
    }
}
