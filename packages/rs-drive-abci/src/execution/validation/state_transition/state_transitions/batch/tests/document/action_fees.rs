//! End-to-end coverage of document action fees (the `actionFees` keyword, protocol version
//! 14): a document type charges a fixed fee in credits for an action, split between the
//! contract's owner pot and its moderators pot. Whoever pays the gas pays the fee: the signer,
//! or the contract owner when they sponsor the gas. The contract owner never pays into their
//! own owner pot, a transition that fails pays no fee, and the credits only ever move, so the
//! sum of all credits changes by the gas alone.

use super::gas_sponsorship::gas_sponsorship_tests::{
    balance_of, process_alone, total_fee, Sponsorship, GAS_SPONSOR_INSUFFICIENT_BALANCE,
    IDENTITY_INSUFFICIENT_BALANCE,
};
use super::*;

mod action_fee_tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel::Recheck;
    use crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::{
        PaidConsensusError, SuccessfulExecution, UnpaidConsensusError,
    };
    use dpp::balances::credits::MAX_CREDITS;
    use dpp::block::epoch::Epoch;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::data_contract::document_type::accessors::{
        DocumentTypeV0MutGetters, DocumentTypeV2Getters,
    };
    use dpp::data_contract::document_type::action_fees::agreement::{
        AgreedFeeMultiplier, DocumentActionFeeAgreement,
    };
    use dpp::data_contract::document_type::action_fees::{
        ActionFeePricing, ContractFeePot, DocumentActionFee,
    };
    use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef};
    use dpp::data_contract::DataContract;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::{platform_value, Value};
    use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use drive::grovedb::Transaction;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use drive::util::batch::GroveDbOpBatch;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    const IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED: u32 = 40218;

    /// What creating a card costs on top of the gas: 0.0001 Dash for the contract owner and
    /// 0.001 Dash for the moderation team.
    const OWNER_PART: Credits = 10_000_000;
    const MODERATORS_PART: Credits = 100_000_000;

    fn card_action_fees(pricing: &str) -> Value {
        platform_value!({
            "pricing": pricing,
            "create": {"owner": OWNER_PART, "moderators": MODERATORS_PART},
        })
    }

    /// Declares `action_fees` on the `document_type_name` document type, on a contract that
    /// keeps a banlist when `moderated`, and parses the document type again so that it carries
    /// them.
    fn declare_action_fees(
        contract: &mut DataContract,
        document_type_name: &str,
        action_fees: Value,
        moderated: bool,
    ) {
        let platform_version = PlatformVersion::latest();
        if moderated {
            contract.set_config(contract.config().clone().with_moderation(Some(
                ContractModerationConfig {
                    banlist: true,
                    suspensions: false,
                    moderators: ContractModerators::ContractOwner,
                    warnings: false,
                },
            )));
        }
        let contract_id = contract.id();
        let config = contract.config().clone();
        let tokens = contract.tokens().clone();
        let document_type = contract
            .document_types_mut()
            .get_mut(document_type_name)
            .expect("expected the document type");
        let mut schema = document_type.schema().clone();
        schema
            .insert("actionFees".to_string(), action_fees)
            .expect("expected to declare the action fees");
        *document_type = DocumentType::try_from_schema(
            contract_id,
            1,
            config.version(),
            document_type_name,
            schema,
            None,
            &tokens,
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected the document type to parse with its action fees");
        assert!(document_type.action_fees().is_some());
    }

    /// The card game of the sponsorship tests, whose card creation also charges an action fee
    fn game(
        offered: GasFeesPaidBy,
        pricing: &'static str,
        owner_credits: Credits,
        user_credits: Credits,
        user_key_budget: Option<Credits>,
    ) -> Sponsorship {
        game_with_gold(
            offered,
            pricing,
            owner_credits,
            user_credits,
            15,
            user_key_budget,
        )
    }

    /// `game` with a user holding `user_gold` gold; a card costs 10
    fn game_with_gold(
        offered: GasFeesPaidBy,
        pricing: &'static str,
        owner_credits: Credits,
        user_credits: Credits,
        user_gold: u64,
        user_key_budget: Option<Credits>,
    ) -> Sponsorship {
        Sponsorship::build_customized(
            PlatformVersion::latest(),
            offered,
            false,
            owner_credits,
            user_credits,
            user_gold,
            user_key_budget,
            move |contract| declare_action_fees(contract, "card", card_action_fees(pricing), true),
        )
    }

    fn pots(setup: &Sponsorship, tx: &Transaction) -> (Credits, Credits) {
        pots_of(
            &setup.platform,
            setup.contract.id(),
            tx,
            setup.platform_version,
        )
    }

    /// Every credit held in a tree: identity balances, pools, and the fee pots among the
    /// prefunded balances.
    fn credits_in_trees(setup: &Sponsorship, tx: &Transaction) -> Credits {
        credits_in_trees_of(&setup.platform, tx, setup.platform_version)
    }

    fn unpaid_codes(result: &StateTransitionExecutionResult) -> Vec<u32> {
        match result {
            UnpaidConsensusError(error) => vec![error.code()],
            other => panic!("expected an unpaid rejection, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_charge_the_signer_the_gas_and_the_fee_and_fill_both_pots() {
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "fixed",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let credits_before = credits_in_trees(&setup, &tx);

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let gas = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.user, &tx),
            dash_to_credits!(0.1) - gas - OWNER_PART - MODERATORS_PART
        );
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1),
            "the owner part waits in the owner pot until it is claimed"
        );
        assert_eq!(pots(&setup, &tx), (OWNER_PART, MODERATORS_PART));
        // The fee only moved, from a balance into the pots: what left the trees is the gas,
        // which reaches the fee pools at the end of the block, and the fee is no part of it.
        assert_eq!(credits_before - credits_in_trees(&setup, &tx), gas);
    }

    #[tokio::test]
    async fn should_scale_a_fee_priced_by_the_fee_multiplier_with_the_epoch_multiplier() {
        for (pricing, scaled) in [("feeMultiplier", true), ("fixed", false)] {
            let setup = game(
                GasFeesPaidBy::DocumentOwner,
                pricing,
                dash_to_credits!(0.1),
                dash_to_credits!(0.1),
                None,
            );
            let mut batch = GroveDbOpBatch::new();
            batch.push(
                Epoch::new(0)
                    .expect("expected epoch 0")
                    .update_fee_multiplier_operation(1_500),
            );
            setup
                .platform
                .drive
                .grove_apply_batch(batch, false, None, &setup.platform_version.drive)
                .expect("expected to set the multiplier of epoch 0");

            let transition = setup
                .card_creation_agreeing(
                    GasFeesPaidBy::DocumentOwner,
                    setup.card_action_fee_agreement(
                        DocumentTransitionActionType::Create,
                        AgreedFeeMultiplier {
                            known_permille: 1_500,
                            increase_tolerance_percent: 0,
                        },
                    ),
                )
                .await;
            // The mempool prices the fee as a block does: a fee priced by the multiplier must
            // reach the execution event with the multiplier read, on this route too.
            assert_eq!(setup.check_tx(&transition), Vec::<u32>::new(), "{pricing}");
            let tx = setup.platform.drive.grove.start_transaction();
            let result = setup.process(&transition, &tx);

            assert_matches!(result, SuccessfulExecution { .. });
            let expected = if scaled {
                (OWNER_PART * 3 / 2, MODERATORS_PART * 3 / 2)
            } else {
                (OWNER_PART, MODERATORS_PART)
            };
            assert_eq!(pots(&setup, &tx), expected, "pricing {pricing}");
        }
    }

    #[tokio::test]
    async fn should_not_charge_the_contract_owner_their_own_owner_part() {
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "fixed",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        // The contract owner holds no gold; give them some to pay for the card.
        add_tokens_to_identity_in(&setup, 15);
        let transition = setup
            .card_creation_by_the_contract_owner(GasFeesPaidBy::DocumentOwner)
            .await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let gas = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - gas - MODERATORS_PART
        );
        assert_eq!(pots(&setup, &tx), (0, MODERATORS_PART));
    }

    /// Gold for the contract owner, who holds none and pays for a card like anybody else
    fn add_tokens_to_identity_in(setup: &Sponsorship, amount: u64) {
        let token_id = calculate_token_id(setup.contract.id().as_bytes(), 0);
        add_tokens_to_identity(
            &setup.platform,
            token_id.into(),
            setup.contract_owner.id(),
            amount,
        );
    }

    #[tokio::test]
    async fn should_charge_a_paying_sponsor_the_fee_too_and_let_an_unfunded_user_act() {
        let setup = game(
            GasFeesPaidBy::ContractOwner,
            "fixed",
            dash_to_credits!(0.1),
            0,
            None,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let gas = total_fee(&result);
        assert_eq!(setup.credits(&setup.user, &tx), 0);
        // The sponsor is the contract owner: they pay the gas and the moderators part, and
        // nothing into their own owner pot.
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - gas - MODERATORS_PART
        );
        assert_eq!(pots(&setup, &tx), (0, MODERATORS_PART));
    }

    #[tokio::test]
    async fn should_refuse_unpaid_when_an_insisted_sponsor_covers_the_gas_but_not_the_fee() {
        // Half the moderators part: plenty for the gas, not for the gas and the fee.
        let setup = game(
            GasFeesPaidBy::ContractOwner,
            "fixed",
            MODERATORS_PART / 2,
            dash_to_credits!(0.1),
            None,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_eq!(
            unpaid_codes(&result),
            vec![GAS_SPONSOR_INSUFFICIENT_BALANCE]
        );
        assert_eq!(pots(&setup, &tx), (0, 0));
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1));
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            MODERATORS_PART / 2
        );
    }

    #[tokio::test]
    async fn should_fall_back_to_the_signer_for_the_gas_and_the_whole_fee_when_a_preferred_sponsor_falls_short(
    ) {
        let setup = game(
            GasFeesPaidBy::PreferContractOwner,
            "fixed",
            MODERATORS_PART / 2,
            dash_to_credits!(0.1),
            None,
        );
        let transition = setup
            .card_creation(GasFeesPaidBy::PreferContractOwner)
            .await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let gas = total_fee(&result);
        // The signer is not the contract owner, so they owe both parts.
        assert_eq!(
            setup.credits(&setup.user, &tx),
            dash_to_credits!(0.1) - gas - OWNER_PART - MODERATORS_PART
        );
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            MODERATORS_PART / 2
        );
        assert_eq!(pots(&setup, &tx), (OWNER_PART, MODERATORS_PART));
    }

    #[tokio::test]
    async fn should_refuse_unpaid_a_signer_who_covers_the_gas_but_not_the_fee() {
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "fixed",
            dash_to_credits!(0.1),
            MODERATORS_PART,
            None,
        );
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_eq!(unpaid_codes(&result), vec![IDENTITY_INSUFFICIENT_BALANCE]);
        assert_eq!(pots(&setup, &tx), (0, 0));
        assert_eq!(setup.credits(&setup.user, &tx), MODERATORS_PART);
    }

    #[tokio::test]
    async fn should_charge_no_fee_for_a_transition_that_fails() {
        // The user holds 5 gold and a card costs 10: the creation fails on the token balance,
        // a paid rejection that bumps the nonce instead of creating the card.
        let setup = game_with_gold(
            GasFeesPaidBy::DocumentOwner,
            "fixed",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            5,
            None,
        );
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, PaidConsensusError { .. });
        let gas = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.user, &tx),
            dash_to_credits!(0.1) - gas,
            "a failed transition pays its gas and nothing else"
        );
        assert_eq!(pots(&setup, &tx), (0, 0));
    }

    #[tokio::test]
    async fn should_count_the_fee_against_the_budget_of_the_signing_key() {
        // A budget that covers the storage of a card but not the fee on top of it.
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "fixed",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            Some(MODERATORS_PART),
        );
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_eq!(
            unpaid_codes(&result),
            vec![IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED]
        );
        assert_eq!(pots(&setup, &tx), (0, 0));
    }
    #[tokio::test]
    async fn should_price_by_the_fee_schedule_in_the_first_block_of_an_epoch() {
        // State transitions execute before the end of the block, where an epoch is initialized,
        // so in the first block of an epoch its tree holds no fee multiplier yet. The fee is
        // then priced by the multiplier the epoch is about to be initialized with.
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "feeMultiplier",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        let schedule_multiplier = setup
            .platform_version
            .fee_version
            .uses_version_fee_multiplier_permille
            .expect("expected the fee schedule to set a multiplier");
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process_in(
            &transition,
            &BlockInfo {
                epoch: Epoch::new(7).expect("expected epoch 7"),
                ..Default::default()
            },
            &tx,
        );

        assert_matches!(result, SuccessfulExecution { .. });
        assert_eq!(
            pots(&setup, &tx),
            (
                OWNER_PART * schedule_multiplier / 1_000,
                MODERATORS_PART * schedule_multiplier / 1_000
            )
        );
    }

    const DOCUMENT_ACTION_FEE_AGREEMENT_NOT_SET: u32 = 40132;
    const DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH: u32 = 40133;
    const DOCUMENT_ACTION_FEE_MULTIPLIER_NOT_TOLERATED: u32 = 40134;

    fn paid_codes(result: &StateTransitionExecutionResult) -> Vec<u32> {
        match result {
            PaidConsensusError { error, .. } => vec![error.code()],
            other => panic!("expected a paid rejection, got {other:?}"),
        }
    }

    /// Sets the fee multiplier of epoch 0, the epoch the harness executes in
    fn set_fee_multiplier(setup: &Sponsorship, fee_multiplier_permille: u64) {
        let mut batch = GroveDbOpBatch::new();
        batch.push(
            Epoch::new(0)
                .expect("expected epoch 0")
                .update_fee_multiplier_operation(fee_multiplier_permille),
        );
        setup
            .platform
            .drive
            .grove_apply_batch(batch, false, None, &setup.platform_version.drive)
            .expect("expected to set the multiplier of epoch 0");
    }

    /// Processes `transition` and expects a paid rejection with `code` that charged the gas
    /// of a nonce bump and no fee.
    fn assert_refused_without_a_fee(setup: &Sponsorship, transition: &StateTransition, code: u32) {
        assert_eq!(setup.check_tx(transition), vec![code]);
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(transition, &tx);

        assert_eq!(paid_codes(&result), vec![code]);
        assert_eq!(
            setup.credits(&setup.user, &tx),
            dash_to_credits!(0.1) - total_fee(&result),
            "a refused transition pays its gas and nothing else"
        );
        assert_eq!(pots(setup, &tx), (0, 0));
    }

    #[tokio::test]
    async fn should_refuse_a_transition_that_does_not_say_what_it_agrees_to_pay() {
        for pricing in ["fixed", "feeMultiplier"] {
            let setup = game(
                GasFeesPaidBy::DocumentOwner,
                pricing,
                dash_to_credits!(0.1),
                dash_to_credits!(0.1),
                None,
            );
            let transition = setup
                .card_creation_agreeing(GasFeesPaidBy::DocumentOwner, None)
                .await;

            assert_refused_without_a_fee(
                &setup,
                &transition,
                DOCUMENT_ACTION_FEE_AGREEMENT_NOT_SET,
            );
        }
    }

    #[tokio::test]
    async fn should_refuse_an_agreement_to_other_amounts_than_the_declared_ones() {
        // What a signer holds when the contract owner changed the fee after they read it:
        // an agreement to the old amounts, higher or lower, for either pot.
        for (owner, moderators) in [
            (OWNER_PART - 1, MODERATORS_PART),
            (OWNER_PART + 1, MODERATORS_PART),
            (OWNER_PART, MODERATORS_PART - 1),
            (OWNER_PART, MODERATORS_PART + 1),
            (MODERATORS_PART, OWNER_PART),
        ] {
            let setup = game(
                GasFeesPaidBy::DocumentOwner,
                "fixed",
                dash_to_credits!(0.1),
                dash_to_credits!(0.1),
                None,
            );
            let transition = setup
                .card_creation_agreeing(
                    GasFeesPaidBy::DocumentOwner,
                    Some(DocumentActionFeeAgreement::for_declared_fee(
                        ActionFeePricing::Fixed,
                        DocumentActionFee { owner, moderators },
                        setup.fee_schedule_multiplier(),
                    )),
                )
                .await;

            assert_refused_without_a_fee(
                &setup,
                &transition,
                DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH,
            );
        }
    }

    #[tokio::test]
    async fn should_refuse_an_agreement_to_another_pricing_than_the_declared_one() {
        for (declared, agreed) in [
            ("fixed", ActionFeePricing::FeeMultiplier),
            ("feeMultiplier", ActionFeePricing::Fixed),
        ] {
            let setup = game(
                GasFeesPaidBy::DocumentOwner,
                declared,
                dash_to_credits!(0.1),
                dash_to_credits!(0.1),
                None,
            );
            let transition = setup
                .card_creation_agreeing(
                    GasFeesPaidBy::DocumentOwner,
                    Some(DocumentActionFeeAgreement::for_declared_fee(
                        agreed,
                        DocumentActionFee {
                            owner: OWNER_PART,
                            moderators: MODERATORS_PART,
                        },
                        setup.fee_schedule_multiplier(),
                    )),
                )
                .await;

            assert_refused_without_a_fee(
                &setup,
                &transition,
                DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH,
            );
        }
    }

    #[tokio::test]
    async fn should_accept_a_fee_multiplier_that_rose_within_the_tolerance() {
        // Signed knowing a multiplier of 1000 and accepting 20% more; the epoch's is 1200.
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "feeMultiplier",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        set_fee_multiplier(&setup, 1_200);
        let transition = setup
            .card_creation_agreeing(
                GasFeesPaidBy::DocumentOwner,
                setup.card_action_fee_agreement(
                    DocumentTransitionActionType::Create,
                    AgreedFeeMultiplier {
                        known_permille: 1_000,
                        increase_tolerance_percent: 20,
                    },
                ),
            )
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        // What is charged follows the multiplier of the epoch, not the one the signer knew.
        assert_eq!(
            pots(&setup, &tx),
            (OWNER_PART * 6 / 5, MODERATORS_PART * 6 / 5)
        );
    }

    #[tokio::test]
    async fn should_refuse_a_fee_multiplier_that_rose_beyond_the_tolerance() {
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "feeMultiplier",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        set_fee_multiplier(&setup, 1_201);
        let transition = setup
            .card_creation_agreeing(
                GasFeesPaidBy::DocumentOwner,
                setup.card_action_fee_agreement(
                    DocumentTransitionActionType::Create,
                    AgreedFeeMultiplier {
                        known_permille: 1_000,
                        increase_tolerance_percent: 20,
                    },
                ),
            )
            .await;

        assert_refused_without_a_fee(
            &setup,
            &transition,
            DOCUMENT_ACTION_FEE_MULTIPLIER_NOT_TOLERATED,
        );
    }

    #[tokio::test]
    async fn should_drop_a_transition_from_the_mempool_once_the_fee_multiplier_outran_its_tolerance(
    ) {
        // Admitted knowing a multiplier of 1000 and accepting 20% more; the epoch's multiplier
        // then moves while the transition waits. A block refuses the transition as a paid
        // nonce bump (`should_refuse_a_fee_multiplier_that_rose_beyond_the_tolerance`), so the
        // recheck judges the agreement again, off the multiplier read now, and drops it first.
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "feeMultiplier",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        let transition = setup
            .card_creation_agreeing(
                GasFeesPaidBy::DocumentOwner,
                setup.card_action_fee_agreement(
                    DocumentTransitionActionType::Create,
                    AgreedFeeMultiplier {
                        known_permille: 1_000,
                        increase_tolerance_percent: 20,
                    },
                ),
            )
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            Vec::<u32>::new(),
            "nothing moved"
        );

        set_fee_multiplier(&setup, 1_200);
        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            Vec::<u32>::new(),
            "a rise within the tolerance keeps the transition"
        );

        set_fee_multiplier(&setup, 1_201);
        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            vec![DOCUMENT_ACTION_FEE_MULTIPLIER_NOT_TOLERATED],
            "a rise beyond the tolerance drops it"
        );
        // What the block would have done with it, had the recheck let it through.
        let tx = setup.platform.drive.grove.start_transaction();
        assert_eq!(
            paid_codes(&setup.process(&transition, &tx)),
            vec![DOCUMENT_ACTION_FEE_MULTIPLIER_NOT_TOLERATED]
        );
        drop(tx);

        set_fee_multiplier(&setup, 900);
        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            Vec::<u32>::new(),
            "a multiplier that fell back is always accepted"
        );
    }

    #[tokio::test]
    async fn should_drop_a_transition_from_the_mempool_once_the_declared_amounts_changed() {
        // A contract update may not change the amounts of a document type yet
        // (`validate_action_fees_unchanged`), so the contract is rewritten in Drive as one
        // will be once a moderation charter sets the moderators part. The recheck reads the
        // contract again through the transformer and judges the agreement against what it
        // declares now.
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "fixed",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        assert_eq!(setup.check_tx_at(&transition, Recheck), Vec::<u32>::new());

        let mut raised = setup.contract.clone();
        declare_action_fees(
            &mut raised,
            "card",
            platform_value!({
                "pricing": "fixed",
                "create": {"owner": OWNER_PART, "moderators": MODERATORS_PART + 1},
            }),
            true,
        );
        raised.increment_version();
        setup
            .platform
            .drive
            .update_contract(
                &raised,
                BlockInfo::default(),
                true,
                None,
                setup.platform_version,
                None,
            )
            .expect("expected to rewrite the contract with the raised moderators part");

        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            vec![DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH]
        );
        let tx = setup.platform.drive.grove.start_transaction();
        assert_eq!(
            paid_codes(&setup.process(&transition, &tx)),
            vec![DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH]
        );
    }

    #[tokio::test]
    async fn should_drop_a_sponsored_transition_from_the_mempool_once_the_fee_multiplier_outran_its_tolerance(
    ) {
        // A signer under the fee minimum who relies on the contract owner's sponsorship is
        // rechecked against the state in full, on another route through check tx; the
        // agreement is judged again on that route too.
        let setup = game(
            GasFeesPaidBy::ContractOwner,
            "feeMultiplier",
            dash_to_credits!(0.1),
            0,
            None,
        );
        let transition = setup
            .card_creation_agreeing(
                GasFeesPaidBy::ContractOwner,
                setup.card_action_fee_agreement(
                    DocumentTransitionActionType::Create,
                    AgreedFeeMultiplier {
                        known_permille: 1_000,
                        increase_tolerance_percent: 20,
                    },
                ),
            )
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        assert_eq!(setup.check_tx_at(&transition, Recheck), Vec::<u32>::new());

        set_fee_multiplier(&setup, 1_200);
        assert_eq!(setup.check_tx_at(&transition, Recheck), Vec::<u32>::new());

        set_fee_multiplier(&setup, 1_201);
        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            vec![DOCUMENT_ACTION_FEE_MULTIPLIER_NOT_TOLERATED]
        );
    }

    #[tokio::test]
    async fn should_accept_a_fee_multiplier_that_fell() {
        let setup = game(
            GasFeesPaidBy::DocumentOwner,
            "feeMultiplier",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        set_fee_multiplier(&setup, 500);
        let transition = setup
            .card_creation_agreeing(
                GasFeesPaidBy::DocumentOwner,
                setup.card_action_fee_agreement(
                    DocumentTransitionActionType::Create,
                    AgreedFeeMultiplier {
                        known_permille: 1_000,
                        increase_tolerance_percent: 0,
                    },
                ),
            )
            .await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        assert_eq!(pots(&setup, &tx), (OWNER_PART / 2, MODERATORS_PART / 2));
    }

    #[tokio::test]
    async fn should_require_the_agreement_of_a_sponsored_transition_too() {
        // The contract owner pays the gas and the fee, and a preference can fall back to the
        // signer, so who pays changes nothing: the transition says what it agrees to.
        let setup = game(
            GasFeesPaidBy::ContractOwner,
            "fixed",
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            None,
        );
        let transition = setup
            .card_creation_agreeing(GasFeesPaidBy::ContractOwner, None)
            .await;

        assert_refused_without_a_fee(&setup, &transition, DOCUMENT_ACTION_FEE_AGREEMENT_NOT_SET);
    }

    #[tokio::test]
    async fn should_ignore_an_agreement_on_an_action_that_charges_nothing() {
        // The harness without action fees: the card type charges nothing for a creation.
        let setup = Sponsorship::build_customized(
            PlatformVersion::latest(),
            GasFeesPaidBy::DocumentOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
            |_| {},
        );
        let transition = setup
            .card_creation_agreeing(
                GasFeesPaidBy::DocumentOwner,
                Some(DocumentActionFeeAgreement::for_declared_fee(
                    ActionFeePricing::Fixed,
                    DocumentActionFee {
                        owner: OWNER_PART,
                        moderators: MODERATORS_PART,
                    },
                    setup.fee_schedule_multiplier(),
                )),
            )
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        assert_eq!(
            setup.credits(&setup.user, &tx),
            dash_to_credits!(0.1) - total_fee(&result)
        );
    }

    #[tokio::test]
    async fn should_refuse_a_base_carrying_an_agreement_at_protocol_version_13() {
        // Version 2 of the document base cannot decode on 4.1 software, so while protocol
        // version 13 is active new software treats a batch carrying one as inactive: the same
        // refusal, charging nothing, that every format added since the fork gets. The version
        // 1 base that protocol version 13 builds keeps working, which
        // `should_ignore_who_is_asked_to_pay_the_gas_at_protocol_version_13` covers.
        let platform_version =
            PlatformVersion::get(13).expect("expected protocol version 13 to exist");
        let setup = Sponsorship::build_customized(
            platform_version,
            GasFeesPaidBy::DocumentOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
            |_| {},
        );
        let (document, entropy) = setup.card_of(&setup.user);
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            setup
                .contract
                .document_type_for_name("card")
                .expect("expected the card document type"),
            entropy.0,
            &setup.user_key,
            2,
            0,
            None,
            &setup.user_signer,
            platform_version,
            Some(StateTransitionCreationOptions {
                base_feature_version: Some(2),
                action_fee_agreement: Some(DocumentActionFeeAgreement::for_declared_fee(
                    ActionFeePricing::Fixed,
                    DocumentActionFee {
                        owner: OWNER_PART,
                        moderators: MODERATORS_PART,
                    },
                    AgreedFeeMultiplier {
                        known_permille: 1_000,
                        increase_tolerance_percent: 0,
                    },
                )),
                ..Default::default()
            }),
        )
        .await
        .expect("expected a batch transition");

        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert!(
            matches!(
                &result,
                StateTransitionExecutionResult::InternalError(message)
                    if message.contains("DocumentsBatch") && message.contains("not active")
            ),
            "expected the batch to be inactive before protocol version 14, got {result:?}"
        );
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1));
    }

    /// What each action on a card costs on top of the gas, all to the contract owner: every
    /// amount is different, so a fee charged for the wrong action shows.
    const CREATE_FEE: Credits = 1_000;
    const REPLACE_FEE: Credits = 2_000;
    const DELETE_FEE: Credits = 3_000;
    const TRANSFER_FEE: Credits = 4_000;
    const UPDATE_PRICE_FEE: Credits = 5_000;
    const PURCHASE_FEE: Credits = 6_000;

    #[tokio::test]
    async fn should_charge_each_action_its_own_fee() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        // A card that can be replaced, priced, purchased, transferred and deleted.
        let mut contract = json_document_to_contract(
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-documents-mutable.json",
            true,
            platform_version,
        )
        .expect("expected the card game contract");
        declare_action_fees(
            &mut contract,
            "card",
            platform_value!({
                "pricing": "fixed",
                "create": {"owner": CREATE_FEE},
                "replace": {"owner": REPLACE_FEE},
                "delete": {"owner": DELETE_FEE},
                "transfer": {"owner": TRANSFER_FEE},
                "update_price": {"owner": UPDATE_PRICE_FEE},
                "purchase": {"owner": PURCHASE_FEE},
            }),
            false,
        );
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
            .expect("expected to apply the contract");
        let card = contract
            .document_type_for_name("card")
            .expect("expected the card document type");

        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1));
        let (buyer, buyer_signer, buyer_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1));

        let mut rng = StdRng::seed_from_u64(433);
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = card
            .random_document_with_identifier_and_entropy(
                &mut rng,
                seller.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document
            .set_id_for_creation(card, &entropy.0, 1, platform_version)
            .expect("expected to set the document id");
        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let tx = platform.drive.grove.start_transaction();
        let state = platform.state.load();
        let owner_pot = |tx: &Transaction| {
            platform
                .drive
                .fetch_contract_fee_pot(
                    contract.id(),
                    ContractFeePot::Owner,
                    Some(tx),
                    platform_version,
                )
                .expect("expected to fetch the owner pot")
                .credits
        };
        // Processes `transition` and returns what it added to the owner pot.
        let charged = |transition: StateTransition, action: &str| {
            let before = owner_pot(&tx);
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")],
                    &state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            assert_matches!(
                result.execution_results().as_slice(),
                [SuccessfulExecution { .. }],
                "{action}"
            );
            owner_pot(&tx) - before
        };
        let by = |identity: &Identity| identity.id();
        let signed_by_the_seller: (&IdentityPublicKey, &SimpleSigner) =
            (&seller_key, &seller_signer);
        let signed_by_the_buyer: (&IdentityPublicKey, &SimpleSigner) = (&buyer_key, &buyer_signer);

        let creation = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            card,
            entropy.0,
            signed_by_the_seller.0,
            1,
            0,
            None,
            signed_by_the_seller.1,
            platform_version,
            agreeing_to(card, DocumentTransitionActionType::Create),
        )
        .await
        .expect("expected the creation");
        assert_eq!(charged(creation, "create"), CREATE_FEE);

        document.set("attack", 5.into());
        document.bump_revision();
        let replacement = BatchTransition::new_document_replacement_transition_from_document(
            document.clone(),
            card,
            signed_by_the_seller.0,
            2,
            0,
            None,
            signed_by_the_seller.1,
            platform_version,
            agreeing_to(card, DocumentTransitionActionType::Replace),
        )
        .await
        .expect("expected the replacement");
        assert_eq!(charged(replacement, "replace"), REPLACE_FEE);

        document.bump_revision();
        let price = dash_to_credits!(0.1);
        let price_update = BatchTransition::new_document_update_price_transition_from_document(
            document.clone(),
            card,
            price,
            signed_by_the_seller.0,
            3,
            0,
            None,
            signed_by_the_seller.1,
            platform_version,
            agreeing_to(card, DocumentTransitionActionType::UpdatePrice),
        )
        .await
        .expect("expected the price update");
        assert_eq!(charged(price_update, "update_price"), UPDATE_PRICE_FEE);

        document.bump_revision();
        let purchase = BatchTransition::new_document_purchase_transition_from_document(
            document.clone(),
            card,
            by(&buyer),
            price,
            signed_by_the_buyer.0,
            1,
            0,
            None,
            signed_by_the_buyer.1,
            platform_version,
            agreeing_to(card, DocumentTransitionActionType::Purchase),
        )
        .await
        .expect("expected the purchase");
        assert_eq!(charged(purchase, "purchase"), PURCHASE_FEE);

        document.set_owner_id(by(&buyer));
        document.bump_revision();
        let transfer = BatchTransition::new_document_transfer_transition_from_document(
            document.clone(),
            card,
            by(&seller),
            signed_by_the_buyer.0,
            2,
            0,
            None,
            signed_by_the_buyer.1,
            platform_version,
            agreeing_to(card, DocumentTransitionActionType::Transfer),
        )
        .await
        .expect("expected the transfer");
        assert_eq!(charged(transfer, "transfer"), TRANSFER_FEE);

        document.set_owner_id(by(&seller));
        document.bump_revision();
        let deletion = BatchTransition::new_document_deletion_transition_from_document(
            document,
            card,
            signed_by_the_seller.0,
            4,
            0,
            None,
            signed_by_the_seller.1,
            platform_version,
            agreeing_to(card, DocumentTransitionActionType::Delete),
        )
        .await
        .expect("expected the deletion");
        assert_eq!(charged(deletion, "delete"), DELETE_FEE);
    }

    fn pots_of(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract_id: Identifier,
        tx: &Transaction,
        platform_version: &PlatformVersion,
    ) -> (Credits, Credits) {
        let pot = |pot| {
            platform
                .drive
                .fetch_contract_fee_pot(contract_id, pot, Some(tx), platform_version)
                .expect("expected to fetch the pot")
                .credits
        };
        (pot(ContractFeePot::Owner), pot(ContractFeePot::Moderators))
    }

    fn credits_in_trees_of(
        platform: &TempPlatform<MockCoreRPCLike>,
        tx: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Credits {
        platform
            .drive
            .calculate_total_credits_balance(Some(tx), &platform_version.drive)
            .expect("expected to sum the credits")
            .total_in_trees()
            .expect("expected the credits to add up")
    }

    /// The agreement to what `document_type` declares for `action`, at the fee multiplier of
    /// the fee schedule
    fn agreeing_to(
        document_type: DocumentTypeRef,
        action: DocumentTransitionActionType,
    ) -> Option<StateTransitionCreationOptions> {
        Some(StateTransitionCreationOptions {
            action_fee_agreement: DocumentActionFeeAgreement::for_document_type_action(
                document_type,
                action,
                AgreedFeeMultiplier {
                    known_permille: 1_000,
                    increase_tolerance_percent: 0,
                },
            ),
            ..Default::default()
        })
    }

    /// What the seller asks for the card they put up for sale: 0.01 Dash
    const CARD_PRICE: Credits = 1_000_000_000;

    /// Who creates the card and puts it up for sale
    #[derive(Clone, Copy)]
    enum Seller {
        ContractOwner,
        User,
    }

    /// A purchase fee of `owner_part` for the contract owner and `MODERATORS_PART` for the
    /// moderation team
    fn purchase_fee(owner_part: Credits) -> Value {
        platform_value!({
            "pricing": "fixed",
            "purchase": {"owner": owner_part, "moderators": MODERATORS_PART},
        })
    }

    /// The card game of the sponsorship tests with a card `seller` created and put up for
    /// sale, committed so that check tx sees it, and the card as a purchase of it carries it.
    /// A purchase charges `purchase_fee` to whoever pays the gas, and its token cost offers
    /// that the contract owner pays it.
    async fn card_for_sale(seller: Seller, purchase_fee: Value) -> (Sponsorship, Document) {
        let setup = Sponsorship::build_customized(
            PlatformVersion::latest(),
            GasFeesPaidBy::DocumentOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
            move |contract| {
                let offered: u8 = GasFeesPaidBy::ContractOwner.into();
                contract
                    .document_types_mut()
                    .get_mut("card")
                    .expect("expected the card document type")
                    .schema_mut()
                    .get_mut("tokenCost")
                    .expect("expected to get the token cost")
                    .expect("expected the token cost to be set")
                    .get_mut("purchase")
                    .expect("expected to get the purchase token cost")
                    .expect("expected the purchase token cost to be set")
                    .set_value("gasFeesPaidBy", offered.into())
                    .expect("expected to offer that the contract owner pays the gas");
                declare_action_fees(contract, "card", purchase_fee, true);
            },
        );
        let (seller, seller_key, seller_signer) = match seller {
            Seller::ContractOwner => {
                // The contract owner pays for their card in gold like anybody else; the user
                // holds gold from the start.
                add_tokens_to_identity_in(&setup, 15);
                (
                    &setup.contract_owner,
                    &setup.contract_owner_key,
                    &setup.contract_owner_signer,
                )
            }
            Seller::User => (&setup.user, &setup.user_key, &setup.user_signer),
        };
        // The price costs the second token.
        add_tokens_to_identity(
            &setup.platform,
            calculate_token_id(setup.contract.id().as_bytes(), 1).into(),
            seller.id(),
            1,
        );
        let card_type = setup
            .contract
            .document_type_for_name("card")
            .expect("expected the card document type");

        let tx = setup.platform.drive.grove.start_transaction();
        let creation = setup
            .card_creation_by(
                seller,
                seller_key,
                seller_signer,
                Some(GasFeesPaidBy::DocumentOwner),
            )
            .await;
        assert_matches!(setup.process(&creation, &tx), SuccessfulExecution { .. });
        let (mut card, _) = setup.card_of(seller);
        card.bump_revision();
        let price_update = BatchTransition::new_document_update_price_transition_from_document(
            card.clone(),
            card_type,
            CARD_PRICE,
            seller_key,
            3,
            0,
            Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                payment_token_contract_id: None,
                token_contract_position: 1,
                minimum_token_cost: None,
                maximum_token_cost: Some(1),
                gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            })),
            seller_signer,
            setup.platform_version,
            None,
        )
        .await
        .expect("expected the price update");
        assert_matches!(
            setup.process(&price_update, &tx),
            SuccessfulExecution { .. }
        );
        setup
            .platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .expect("expected to commit the card for sale");
        card.bump_revision();
        (setup, card)
    }

    /// The user's purchase of `card`, asking `requested` for the gas
    async fn purchase_of(
        setup: &Sponsorship,
        card: Document,
        requested: GasFeesPaidBy,
    ) -> StateTransition {
        purchase_by(
            setup,
            card,
            (&setup.user, &setup.user_key, &setup.user_signer),
            requested,
        )
        .await
    }

    /// `buyer`'s purchase of `card`, with their first identity contract nonce, asking
    /// `requested` for the gas
    async fn purchase_by(
        setup: &Sponsorship,
        card: Document,
        (buyer, buyer_key, buyer_signer): (&Identity, &IdentityPublicKey, &SimpleSigner),
        requested: GasFeesPaidBy,
    ) -> StateTransition {
        let card_type = setup
            .contract
            .document_type_for_name("card")
            .expect("expected the card document type");
        BatchTransition::new_document_purchase_transition_from_document(
            card,
            card_type,
            buyer.id(),
            CARD_PRICE,
            buyer_key,
            2,
            0,
            Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                payment_token_contract_id: None,
                token_contract_position: 0,
                minimum_token_cost: None,
                maximum_token_cost: Some(3),
                gas_fees_paid_by: requested,
            })),
            buyer_signer,
            setup.platform_version,
            agreeing_to(card_type, DocumentTransitionActionType::Purchase),
        )
        .await
        .expect("expected the purchase")
    }

    /// A purchase moves its price out of the buyer's balance with its own operations, and its
    /// action fee leaves the same balance: the buyer pays both, and the credits only move.
    #[tokio::test]
    async fn should_charge_the_buyer_both_the_price_and_the_purchase_fee() {
        let (setup, card) = card_for_sale(Seller::ContractOwner, purchase_fee(OWNER_PART)).await;
        let purchase = purchase_of(&setup, card, GasFeesPaidBy::DocumentOwner).await;
        assert_check_tx_valid_at_all_levels(
            &setup.platform,
            &purchase
                .serialize_to_bytes()
                .expect("expected to serialize"),
            "a purchase owing a purchase fee",
        );
        let tx = setup.platform.drive.grove.start_transaction();
        let buyer_before = setup.credits(&setup.user, &tx);
        let seller_before = setup.credits(&setup.contract_owner, &tx);
        let credits_before = credits_in_trees(&setup, &tx);

        let result = setup.process(&purchase, &tx);

        let SuccessfulExecution { fee_result, .. } = &result else {
            panic!("expected the purchase to execute, got {result:?}");
        };
        let gas = fee_result.total_base_fee();
        let refunded_to_seller = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.contract_owner.id())
            .unwrap_or_default();
        assert_eq!(
            setup.credits(&setup.user, &tx),
            buyer_before - CARD_PRICE - OWNER_PART - MODERATORS_PART - gas,
            "the buyer pays the price, the purchase fee and the gas"
        );
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            seller_before + CARD_PRICE + refunded_to_seller
        );
        assert_eq!(pots(&setup, &tx), (OWNER_PART, MODERATORS_PART));
        // Only the gas leaves the trees, less what the purchase refunded the seller for the
        // storage they had paid; the price and the fee only move.
        assert_eq!(
            credits_in_trees(&setup, &tx),
            credits_before - gas + refunded_to_seller
        );
    }

    /// A purchase from the contract owner whose gas they sponsor pays them the price with its
    /// own operations, and takes the moderators part of the purchase fee from the same
    /// balance: the owner gets the price less that part and the gas, and the credits only move.
    #[tokio::test]
    async fn should_pay_a_seller_who_sponsors_the_gas_the_price_less_the_moderators_part_and_the_gas(
    ) {
        purchase_from_a_seller_who_sponsors_the_gas(OWNER_PART).await;
    }

    /// Fee validation estimates the signer paying the whole fee before it settles on the
    /// sponsor, who is never charged the owner part. With an owner part that leaves the price
    /// no room below the largest balance there can be, the price and that fee would be one
    /// removal no balance covers: the estimate must still admit the purchase the sponsor pays.
    #[tokio::test]
    async fn should_admit_a_sponsored_purchase_whose_waived_owner_part_no_signer_could_pay() {
        purchase_from_a_seller_who_sponsors_the_gas(MAX_CREDITS - MODERATORS_PART - 1).await;
    }

    /// The gas fee validation estimates for `transition` when a block executes it, and whether
    /// the contract owner paid it, read off an execution in a transaction that is then thrown
    /// away. Check tx estimates differently: it does not run a batch's state validation.
    fn estimated_at_execution(
        setup: &Sponsorship,
        transition: &StateTransition,
    ) -> (Credits, bool) {
        let tx = setup.platform.drive.grove.start_transaction();
        let owner_before = setup.credits(&setup.contract_owner, &tx);
        match setup.process(transition, &tx) {
            SuccessfulExecution {
                estimated_fees: Some(estimated_fees),
                ..
            } => (
                estimated_fees.total_base_fee(),
                setup.credits(&setup.contract_owner, &tx) < owner_before,
            ),
            other => panic!("expected an execution with an estimated fee, got {other:?}"),
        }
    }

    /// Takes `credits` out of the contract owner's committed balance
    fn take_from_the_contract_owner(setup: &Sponsorship, credits: Credits) {
        setup
            .platform
            .drive
            .remove_from_identity_balance(
                setup.contract_owner.id().to_buffer(),
                credits,
                &BlockInfo::default(),
                true,
                None,
                setup.platform_version,
                None,
            )
            .expect("expected to take credits from the contract owner");
    }

    /// Execution charges the payer fee validation settled on. A purchase from a third party
    /// that owes only a moderators part is estimated lower with the buyer paying, whose fee
    /// merges into the price leaving them, than with the sponsor paying, whose fee is a write
    /// of its own. A sponsor one credit short of their own estimate is passed over by fee
    /// validation, which returns the buyer's lower estimate: the sponsor would cover that one,
    /// so asking the sponsor question again on it would charge them. Execution charges the buyer.
    #[tokio::test]
    async fn should_charge_the_buyer_when_the_sponsor_covers_only_the_buyers_estimate() {
        let (mut setup, card) = card_for_sale(
            Seller::User,
            platform_value!({
                "pricing": "fixed",
                "purchase": {"moderators": MODERATORS_PART},
            }),
        )
        .await;
        let (buyer, buyer_signer, buyer_key) =
            setup_identity(&mut setup.platform, 777, dash_to_credits!(0.1));
        add_tokens_to_identity(
            &setup.platform,
            calculate_token_id(setup.contract.id().as_bytes(), 0).into(),
            buyer.id(),
            3,
        );
        let purchase = purchase_by(
            &setup,
            card,
            (&buyer, &buyer_key, &buyer_signer),
            GasFeesPaidBy::PreferContractOwner,
        )
        .await;

        // Settle the contract owner on a balance that covers the gas estimated with them paying
        // and the moderators part with nothing to spare. That gas includes reading their
        // balance, which costs less as the balance shrinks, so it is estimated again at each
        // balance left, with the owner paying, until the two agree.
        let mut owner_balance = {
            let tx = setup.platform.drive.grove.start_transaction();
            setup.credits(&setup.contract_owner, &tx)
        };
        let mut sponsor_estimate = None;
        for _ in 0..5 {
            let (estimate, sponsor_paid) = estimated_at_execution(&setup, &purchase);
            assert!(
                sponsor_paid,
                "expected the contract owner to pay with {owner_balance}"
            );
            let just_enough = estimate + MODERATORS_PART;
            if just_enough == owner_balance {
                sponsor_estimate = Some(estimate);
                break;
            }
            take_from_the_contract_owner(&setup, owner_balance - just_enough);
            owner_balance = just_enough;
        }
        let sponsor_estimate =
            sponsor_estimate.expect("expected the contract owner's balance to settle");
        // One credit less, and the owner falls short of paying.
        take_from_the_contract_owner(&setup, 1);
        let short_of_paying = owner_balance - 1;
        assert_check_tx_valid_at_all_levels(
            &setup.platform,
            &purchase
                .serialize_to_bytes()
                .expect("expected to serialize"),
            "a purchase whose preferred sponsor falls one credit short",
        );

        let tx = setup.platform.drive.grove.start_transaction();
        let buyer_before = balance_of(&setup.platform, &buyer, &tx, setup.platform_version);
        let seller_before = setup.credits(&setup.user, &tx);

        let result = setup.process(&purchase, &tx);

        let SuccessfulExecution {
            fee_result,
            estimated_fees,
            ..
        } = &result
        else {
            panic!("expected the purchase to execute, got {result:?}");
        };
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            short_of_paying,
            "the passed over sponsor pays nothing"
        );
        let buyer_estimate = estimated_fees
            .as_ref()
            .map(|estimate| estimate.total_base_fee())
            .expect("expected fee validation to return an estimate");
        assert!(
            buyer_estimate < sponsor_estimate
                && buyer_estimate + MODERATORS_PART <= short_of_paying,
            "expected fee validation to return the buyer's estimate, which the sponsor covers"
        );
        let gas = fee_result.total_base_fee();
        let refunded_to_seller = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.user.id())
            .unwrap_or_default();
        assert_eq!(
            balance_of(&setup.platform, &buyer, &tx, setup.platform_version),
            buyer_before - CARD_PRICE - MODERATORS_PART - gas,
            "the buyer pays the price, the fee and the gas"
        );
        assert_eq!(
            setup.credits(&setup.user, &tx),
            seller_before + CARD_PRICE + refunded_to_seller
        );
        assert_eq!(pots(&setup, &tx), (0, MODERATORS_PART));
    }

    /// A buyer who pays the gas owes the price and the whole purchase fee, which fee validation
    /// knows without an estimate. When no balance could fund them, the purchase is refused
    /// unpaid for an insufficient balance before anything is estimated, rather than failing an
    /// estimate that would merge them into one removal no balance covers.
    #[tokio::test]
    async fn should_refuse_unpaid_a_buyer_who_cannot_fund_the_price_and_the_purchase_fee() {
        let (setup, card) = card_for_sale(
            Seller::ContractOwner,
            purchase_fee(MAX_CREDITS - MODERATORS_PART - 1),
        )
        .await;
        let purchase = purchase_of(&setup, card, GasFeesPaidBy::DocumentOwner).await;
        assert_eq!(
            setup.check_tx(&purchase),
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );
        let tx = setup.platform.drive.grove.start_transaction();
        let buyer_before = setup.credits(&setup.user, &tx);

        let result = setup.process(&purchase, &tx);

        assert_eq!(unpaid_codes(&result), vec![IDENTITY_INSUFFICIENT_BALANCE]);
        assert_eq!(setup.credits(&setup.user, &tx), buyer_before);
        assert_eq!(pots(&setup, &tx), (0, 0));
    }

    /// The user buys the card for sale, with an owner part of `owner_part`, from the contract
    /// owner, who sponsors the gas; checks it through check tx and what execution moves
    async fn purchase_from_a_seller_who_sponsors_the_gas(owner_part: Credits) {
        let (setup, card) = card_for_sale(Seller::ContractOwner, purchase_fee(owner_part)).await;
        let purchase = purchase_of(&setup, card, GasFeesPaidBy::ContractOwner).await;
        assert_check_tx_valid_at_all_levels(
            &setup.platform,
            &purchase
                .serialize_to_bytes()
                .expect("expected to serialize"),
            "a sponsored purchase from the sponsor owing a purchase fee",
        );
        let tx = setup.platform.drive.grove.start_transaction();
        let buyer_before = setup.credits(&setup.user, &tx);
        let seller_before = setup.credits(&setup.contract_owner, &tx);
        let credits_before = credits_in_trees(&setup, &tx);

        let result = setup.process(&purchase, &tx);

        let SuccessfulExecution { fee_result, .. } = &result else {
            panic!("expected the purchase to execute, got {result:?}");
        };
        let gas = fee_result.total_base_fee();
        let refunded_to_seller = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.contract_owner.id())
            .unwrap_or_default();
        assert_eq!(
            setup.credits(&setup.user, &tx),
            buyer_before - CARD_PRICE,
            "the buyer pays the price only"
        );
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            seller_before + CARD_PRICE + refunded_to_seller - MODERATORS_PART - gas,
            "the seller gets the price and pays the moderators part and the gas"
        );
        assert_eq!(
            pots(&setup, &tx),
            (0, MODERATORS_PART),
            "the contract owner never pays into their own owner pot"
        );
        assert_eq!(
            credits_in_trees(&setup, &tx),
            credits_before - gas + refunded_to_seller
        );
    }

    /// A contested document's creation moves the prefunded voting balance out of its creator's
    /// balance with its own operations, and its action fee leaves the same balance: the
    /// creator pays both, and the credits only move.
    #[tokio::test]
    async fn should_charge_the_creator_of_a_contested_document_both_the_voting_fund_and_the_creation_fee(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        // A copy of DPNS under another id: its domains are contested, and no data trigger
        // asks for a preorder.
        let mut contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            true,
            platform_version,
        )
        .expect("expected the contested contract");
        declare_action_fees(
            &mut contract,
            "domain",
            platform_value!({
                "pricing": "fixed",
                "create": {"owner": OWNER_PART, "moderators": MODERATORS_PART},
            }),
            true,
        );
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
            .expect("expected to apply the contract");
        let domain = contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        let (creator, signer, key) = setup_identity(&mut platform, 93, dash_to_credits!(1));

        let mut rng = StdRng::seed_from_u64(433);
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document
            .set_id_for_creation(domain, &entropy.0, 1, platform_version)
            .expect("expected to set the document id");
        document.set("parentDomainName", "dash".into());
        document.set("normalizedParentDomainName", "dash".into());
        document.set("label", "quantum".into());
        document.set("normalizedLabel", "quantum".into());
        document.set("records.identity", creator.id().into());
        document.set("subdomainRules.allowSubdomains", false.into());

        let creation = BatchTransition::new_document_creation_transition_from_document(
            document,
            domain,
            entropy.0,
            &key,
            1,
            0,
            None,
            &signer,
            platform_version,
            agreeing_to(domain, DocumentTransitionActionType::Create),
        )
        .await
        .expect("expected the creation");
        let voting_fund = platform_version
            .fee_version
            .vote_resolution_fund_fees
            .contested_document_vote_resolution_fund_required_amount;

        assert_check_tx_valid_at_all_levels(
            &platform,
            &creation
                .serialize_to_bytes()
                .expect("expected to serialize"),
            "a contested creation owing a creation fee",
        );
        let tx = platform.drive.grove.start_transaction();
        let creator_before = balance_of(&platform, &creator, &tx, platform_version);
        let credits_before = credits_in_trees_of(&platform, &tx, platform_version);

        let result = process_alone(
            &platform,
            &creation,
            &BlockInfo::default(),
            &tx,
            platform_version,
        );

        let SuccessfulExecution { fee_result, .. } = &result else {
            panic!("expected the creation to execute, got {result:?}");
        };
        let gas = fee_result.total_base_fee();
        assert_eq!(
            balance_of(&platform, &creator, &tx, platform_version),
            creator_before - voting_fund - OWNER_PART - MODERATORS_PART - gas,
            "the creator pays the voting fund, the creation fee and the gas"
        );
        assert_eq!(
            pots_of(&platform, contract.id(), &tx, platform_version),
            (OWNER_PART, MODERATORS_PART)
        );
        // Only the gas leaves the trees; the voting fund and the fee only move.
        assert_eq!(
            credits_in_trees_of(&platform, &tx, platform_version),
            credits_before - gas
        );
    }
}
