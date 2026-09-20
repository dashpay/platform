//! End-to-end coverage of document action fees (the `actionFees` keyword, protocol version
//! 14): a document type charges a fixed fee in credits for an action, split between the
//! contract's owner pot and its moderators pot. Whoever pays the gas pays the fee: the signer,
//! or the contract owner when they sponsor the gas. The contract owner never pays into their
//! own owner pot, a transition that fails pays no fee, and the credits only ever move, so the
//! sum of all credits changes by the gas alone.

use super::gas_sponsorship::gas_sponsorship_tests::{
    total_fee, Sponsorship, GAS_SPONSOR_INSUFFICIENT_BALANCE, IDENTITY_INSUFFICIENT_BALANCE,
};
use super::*;

mod action_fee_tests {
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::{
        PaidConsensusError, SuccessfulExecution, UnpaidConsensusError,
    };
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
    use dpp::data_contract::document_type::action_fees::ContractFeePot;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::{platform_value, Value};
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::drive::contract::fee_pots::types::ContractFeePotState;
    use drive::grovedb::Transaction;

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

    /// Declares `action_fees` on the `card` document type of a contract that keeps a banlist,
    /// and parses the document type again so that it carries them.
    fn declare_action_fees(contract: &mut DataContract, action_fees: Value) {
        let platform_version = PlatformVersion::latest();
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
            },
        )));
        let contract_id = contract.id();
        let config = contract.config().clone();
        let tokens = contract.tokens().clone();
        let card = contract
            .document_types_mut()
            .get_mut("card")
            .expect("expected the card document type");
        let mut schema = card.schema().clone();
        schema
            .insert("actionFees".to_string(), action_fees)
            .expect("expected to declare the action fees");
        *card = DocumentType::try_from_schema(
            contract_id,
            1,
            config.version(),
            "card",
            schema,
            None,
            &tokens,
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected the card document type to parse with its action fees");
        assert!(card.action_fees().is_some());
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
            move |contract| declare_action_fees(contract, card_action_fees(pricing)),
        )
    }

    fn pot(setup: &Sponsorship, pot: ContractFeePot, tx: &Transaction) -> ContractFeePotState {
        setup
            .platform
            .drive
            .fetch_contract_fee_pot(setup.contract.id(), pot, Some(tx), setup.platform_version)
            .expect("expected to fetch the pot")
    }

    fn pots(setup: &Sponsorship, tx: &Transaction) -> (Credits, Credits) {
        (
            pot(setup, ContractFeePot::Owner, tx).credits,
            pot(setup, ContractFeePot::Moderators, tx).credits,
        )
    }

    /// Every credit held in a tree: identity balances, pools, and the fee pots among the
    /// prefunded balances.
    fn credits_in_trees(setup: &Sponsorship, tx: &Transaction) -> Credits {
        setup
            .platform
            .drive
            .calculate_total_credits_balance(Some(tx), &setup.platform_version.drive)
            .expect("expected to sum the credits")
            .total_in_trees()
            .expect("expected the credits to add up")
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
        use dpp::block::epoch::Epoch;
        use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
        use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use drive::util::batch::GroveDbOpBatch;

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

            let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
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
        let token_id = dpp::tokens::calculate_token_id(setup.contract.id().as_bytes(), 0);
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
}
