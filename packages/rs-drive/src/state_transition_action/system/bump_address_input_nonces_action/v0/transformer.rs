use std::collections::BTreeMap;
use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use crate::error::Error;
use crate::state_transition_action::address_funds::restore_input_spends_for_failed_transition;
use crate::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNoncesActionV0;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;

impl BumpAddressInputNoncesActionV0 {
    // IdentityCreateFromAddresses transformers

    /// from a failed IdentityCreateFromAddresses transition, given the balances its inputs keep
    /// after the spend (the balances its action was built with)
    ///
    /// A failed identity creation spends nothing, so each input keeps its whole balance: what
    /// remains after the spend plus the amount it would have spent. Nothing is deducted here: the
    /// fee and `penalty_credits` are taken from these balances and booked to the fee pools
    /// together, in the order of the transition's fee strategy followed by the inputs it does not
    /// name.
    pub fn from_failed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        inputs_with_remaining_balance: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        penalty_credits: Credits,
    ) -> Result<Self, Error> {
        let mut inputs_with_balance = inputs_with_remaining_balance.clone();
        restore_input_spends_for_failed_transition(&mut inputs_with_balance, &value.inputs)?;

        let mut fee_strategy = value.fee_strategy.clone();
        for index in (0..inputs_with_balance.len()).filter_map(|index| u16::try_from(index).ok()) {
            let step = AddressFundsFeeStrategyStep::DeductFromInput(index);
            if !fee_strategy.contains(&step) {
                fee_strategy.push(step);
            }
        }

        Ok(BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: inputs_with_balance,
            fee_strategy,
            user_fee_increase: value.user_fee_increase,
            penalty_credits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
    use dpp::address_funds::PlatformAddress;

    const TEST_FEE: u16 = 7;
    const TEST_PENALTY: Credits = 900;

    fn make_fee_strategy() -> AddressFundsFeeStrategy {
        vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(1),
        ]
    }

    fn make_identity_create_from_addresses_transition(
        fee_strategy: AddressFundsFeeStrategy,
    ) -> IdentityCreateFromAddressesTransitionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (11_u32, 1000_u64));
        inputs.insert(PlatformAddress::P2sh([0xBB; 20]), (21_u32, 400_u64));
        IdentityCreateFromAddressesTransitionV0 {
            public_keys: vec![],
            inputs,
            output: None,
            fee_strategy,
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        }
    }

    /// The balances left after the transition's spend, as its action carries them.
    fn remaining_after_spend() -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        let mut remaining = BTreeMap::new();
        remaining.insert(PlatformAddress::P2pkh([0xAA; 20]), (11_u32, 4000_u64));
        remaining.insert(PlatformAddress::P2sh([0xBB; 20]), (21_u32, 2600_u64));
        remaining
    }

    #[test]
    fn should_give_a_failed_identity_create_from_addresses_its_whole_input_balances() {
        let transition = make_identity_create_from_addresses_transition(make_fee_strategy());
        let action =
            BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition(
                &transition,
                &remaining_after_spend(),
                TEST_PENALTY,
            )
            .expect("the balances cover the inputs");

        // The remaining balance plus the amount the input would have spent, nothing deducted.
        let mut expected = BTreeMap::new();
        expected.insert(PlatformAddress::P2pkh([0xAA; 20]), (11_u32, 5000_u64));
        expected.insert(PlatformAddress::P2sh([0xBB; 20]), (21_u32, 3000_u64));
        assert_eq!(action.inputs_with_remaining_balance, expected);
        assert_eq!(action.fee_strategy, make_fee_strategy());
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.penalty_credits, TEST_PENALTY);
    }

    #[test]
    fn should_let_every_input_of_a_failed_identity_create_from_addresses_pay_its_fee() {
        let mut transition = make_identity_create_from_addresses_transition(vec![
            AddressFundsFeeStrategyStep::ReduceOutput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(1),
        ]);
        transition
            .inputs
            .insert(PlatformAddress::P2sh([0xCC; 20]), (31_u32, 100_u64));
        let mut remaining = remaining_after_spend();
        remaining.insert(PlatformAddress::P2sh([0xCC; 20]), (31_u32, 0_u64));

        let action =
            BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition(
                &transition,
                &remaining,
                TEST_PENALTY,
            )
            .expect("the balances cover the inputs");

        // The transition's own steps first, then the inputs it does not name, in input order.
        assert_eq!(
            action.fee_strategy,
            vec![
                AddressFundsFeeStrategyStep::ReduceOutput(0),
                AddressFundsFeeStrategyStep::DeductFromInput(1),
                AddressFundsFeeStrategyStep::DeductFromInput(0),
                AddressFundsFeeStrategyStep::DeductFromInput(2),
            ]
        );
    }

    #[test]
    fn should_refuse_balances_that_do_not_match_the_inputs() {
        let transition = make_identity_create_from_addresses_transition(make_fee_strategy());

        let mut missing_one = remaining_after_spend();
        missing_one.remove(&PlatformAddress::P2sh([0xBB; 20]));
        assert!(
            BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition(
                &transition,
                &missing_one,
                TEST_PENALTY,
            )
            .is_err()
        );

        let mut other_address = missing_one;
        other_address.insert(PlatformAddress::P2sh([0xDD; 20]), (21_u32, 2600_u64));
        assert!(
            BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition(
                &transition,
                &other_address,
                TEST_PENALTY,
            )
            .is_err()
        );
    }

    #[test]
    fn should_refuse_a_whole_balance_beyond_the_credit_range() {
        let transition = make_identity_create_from_addresses_transition(make_fee_strategy());
        let mut remaining = remaining_after_spend();
        remaining.insert(PlatformAddress::P2pkh([0xAA; 20]), (11_u32, u64::MAX));

        assert!(
            BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition(
                &transition,
                &remaining,
                TEST_PENALTY,
            )
            .is_err()
        );
    }
}
