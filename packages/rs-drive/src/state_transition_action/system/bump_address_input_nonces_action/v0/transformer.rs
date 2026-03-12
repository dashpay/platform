use std::collections::{BTreeMap, HashSet};
use dpp::address_funds::fee_strategy::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use crate::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNoncesActionV0;
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

/// Helper function to subtract penalty credits from input balances.
/// The penalty is distributed across inputs according to the fee strategy order,
/// deducting as much as possible from each input before moving to the next.
fn deduct_penalty_from_inputs(
    inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    fee_strategy: &AddressFundsFeeStrategy,
    penalty_credits: Credits,
) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
    let mut remaining_penalty = penalty_credits;
    let mut result = inputs.clone();

    // Convert inputs to a Vec for index-based access
    let input_keys: Vec<_> = inputs.keys().collect();

    // Track which input indices we've already processed
    let mut processed_indices = HashSet::new();

    // First, process inputs in fee strategy order
    for step in fee_strategy {
        if let AddressFundsFeeStrategyStep::DeductFromInput(idx) = step {
            let idx = *idx as usize;
            if idx < input_keys.len() && !processed_indices.contains(&idx) {
                processed_indices.insert(idx);
                let key = input_keys[idx];
                if let Some((_nonce, balance)) = result.get_mut(key) {
                    let deduction = remaining_penalty.min(*balance);
                    remaining_penalty -= deduction;
                    *balance -= deduction;
                }
            }
        }
    }

    // Then, process any remaining inputs not covered by the fee strategy
    for (idx, key) in input_keys.iter().enumerate() {
        if !processed_indices.contains(&idx) {
            if let Some((_, balance)) = result.get_mut(*key) {
                let deduction = remaining_penalty.min(*balance);
                remaining_penalty -= deduction;
                *balance -= deduction;
            }
        }
    }

    result
}

impl BumpAddressInputNoncesActionV0 {
    /// Helper to create action with penalty deduction
    fn new_with_penalty(
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        fee_strategy: &AddressFundsFeeStrategy,
        penalty_credits: Credits,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: deduct_penalty_from_inputs(
                inputs,
                fee_strategy,
                penalty_credits,
            ),
            fee_strategy: fee_strategy.clone(),
            user_fee_increase,
        }
    }

    // Generic constructor from pre-computed inputs

    /// from inputs with remaining balance (used by shield transitions where
    /// the remaining balances are computed by the processing pipeline)
    pub fn from_inputs_with_remaining_balance(
        inputs_with_remaining_balance: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        fee_strategy: &AddressFundsFeeStrategy,
        penalty_credits: Credits,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        Self::new_with_penalty(
            inputs_with_remaining_balance,
            fee_strategy,
            penalty_credits,
            user_fee_increase,
        )
    }

    // IdentityCreateFromAddresses transformers

    /// from borrowed IdentityCreateFromAddresses transition
    /// Subtracts penalty_credits from the input balances (distributed across inputs according to fee strategy)
    pub fn from_borrowed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    /// from borrowed IdentityCreateFromAddresses transition action
    pub fn from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    // IdentityTopUpFromAddresses transformers

    /// from borrowed IdentityTopUpFromAddresses transition
    pub fn from_borrowed_identity_topup_from_addresses_transition(
        value: &IdentityTopUpFromAddressesTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    /// from borrowed IdentityTopUpFromAddresses transition action
    pub fn from_borrowed_identity_topup_from_addresses_transition_action(
        value: &IdentityTopUpFromAddressesTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    // AddressFundsTransfer transformers

    /// from borrowed AddressFundsTransfer transition
    pub fn from_borrowed_address_funds_transfer_transition(
        value: &AddressFundsTransferTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    /// from borrowed AddressFundsTransfer transition action
    pub fn from_borrowed_address_funds_transfer_transition_action(
        value: &AddressFundsTransferTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::address_funds::PlatformAddress;
    use dpp::identifier::Identifier;

    const TEST_FEE: u16 = 7;

    fn make_inputs() -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (10_u32, 5000_u64));
        inputs.insert(PlatformAddress::P2sh([0xBB; 20]), (20_u32, 3000_u64));
        inputs
    }

    fn make_fee_strategy() -> AddressFundsFeeStrategy {
        vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(1),
        ]
    }

    // ---- deduct_penalty_from_inputs tests ----

    #[test]
    fn test_deduct_penalty_from_inputs_zero_penalty() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let result = deduct_penalty_from_inputs(&inputs, &strategy, 0);
        // No deduction, balances unchanged
        for (addr, (nonce, credits)) in &inputs {
            let (r_nonce, r_credits) = result.get(addr).unwrap();
            assert_eq!(*r_nonce, *nonce);
            assert_eq!(*r_credits, *credits);
        }
    }

    #[test]
    fn test_deduct_penalty_from_inputs_partial_from_first() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        // Deduct 1000 from first input (which has 5000)
        let result = deduct_penalty_from_inputs(&inputs, &strategy, 1000);
        let addr0 = inputs.keys().next().unwrap();
        let (_, balance0) = result.get(addr0).unwrap();
        assert_eq!(*balance0, inputs.get(addr0).unwrap().1 - 1000);
    }

    #[test]
    fn test_deduct_penalty_from_inputs_overflow_first_into_second() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let input_keys: Vec<_> = inputs.keys().cloned().collect();
        let first_balance = inputs.get(&input_keys[0]).unwrap().1;
        // Deduct more than the first input has, remainder goes to second
        let penalty = first_balance + 500;
        let result = deduct_penalty_from_inputs(&inputs, &strategy, penalty);
        let (_, balance0) = result.get(&input_keys[0]).unwrap();
        assert_eq!(*balance0, 0);
        let second_balance = inputs.get(&input_keys[1]).unwrap().1;
        let (_, balance1) = result.get(&input_keys[1]).unwrap();
        assert_eq!(*balance1, second_balance - 500);
    }

    #[test]
    fn test_deduct_penalty_from_inputs_exceeds_all() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        // Deduct more than total balance
        let total: Credits = inputs.values().map(|(_, c)| c).sum();
        let result = deduct_penalty_from_inputs(&inputs, &strategy, total + 1000);
        for (_, credits) in result.values() {
            assert_eq!(*credits, 0);
        }
    }

    #[test]
    fn test_deduct_penalty_empty_strategy_uses_remaining_inputs() {
        let inputs = make_inputs();
        let empty_strategy: AddressFundsFeeStrategy = vec![];
        let result = deduct_penalty_from_inputs(&inputs, &empty_strategy, 1000);
        let total_remaining: Credits = result.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 1000);
    }

    // ---- from_inputs_with_remaining_balance ----

    #[test]
    fn test_from_inputs_with_remaining_balance() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let action = BumpAddressInputNoncesActionV0::from_inputs_with_remaining_balance(
            &inputs, &strategy, 500, TEST_FEE,
        );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        // Verify penalty was applied
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 500);
    }

    #[test]
    fn test_from_inputs_with_remaining_balance_no_penalty() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let action = BumpAddressInputNoncesActionV0::from_inputs_with_remaining_balance(
            &inputs, &strategy, 0, TEST_FEE,
        );
        // No penalty: balances should be identical
        assert_eq!(action.inputs_with_remaining_balance, inputs);
    }

    // ---- IdentityCreateFromAddresses ----

    #[test]
    fn test_from_borrowed_identity_create_from_addresses_transition() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let transition = IdentityCreateFromAddressesTransitionV0 {
            public_keys: vec![],
            inputs: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition(
                &transition, 200,
            );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 200);
    }

    #[test]
    fn test_from_borrowed_identity_create_from_addresses_transition_action() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let action_v0 = IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            public_keys: vec![],
            identity_id: Identifier::from([0xCC; 32]),
            fund_identity_amount: 1000,
            user_fee_increase: TEST_FEE,
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition_action(
                &action_v0, 300,
            );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 300);
    }

    // ---- IdentityTopUpFromAddresses ----

    #[test]
    fn test_from_borrowed_identity_topup_from_addresses_transition() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let transition = IdentityTopUpFromAddressesTransitionV0 {
            inputs: inputs.clone(),
            output: None,
            identity_id: Identifier::from([0xCC; 32]),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition(
                &transition, 100,
            );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 100);
    }

    #[test]
    fn test_from_borrowed_identity_topup_from_addresses_transition_action() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let action_v0 = IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: None,
            fee_strategy: strategy.clone(),
            identity_id: Identifier::from([0xCC; 32]),
            topup_amount: 500,
            user_fee_increase: TEST_FEE,
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition_action(
                &action_v0, 250,
            );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 250);
    }

    // ---- AddressFundsTransfer ----

    #[test]
    fn test_from_borrowed_address_funds_transfer_transition() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let transition = AddressFundsTransferTransitionV0 {
            inputs: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition(
                &transition, 400,
            );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 400);
    }

    #[test]
    fn test_from_borrowed_address_funds_transfer_transition_action() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let action_v0 = AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy.clone(),
            user_fee_increase: TEST_FEE,
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition_action(
                &action_v0, 600,
            );
        assert_eq!(action.user_fee_increase, TEST_FEE);
        assert_eq!(action.fee_strategy, strategy);
        let total_remaining: Credits =
            action.inputs_with_remaining_balance.values().map(|(_, c)| c).sum();
        let total_original: Credits = inputs.values().map(|(_, c)| c).sum();
        assert_eq!(total_remaining, total_original - 600);
    }

    // ---- nonces are preserved ----

    #[test]
    fn test_nonces_are_preserved_from_transition() {
        let inputs = make_inputs();
        let strategy = make_fee_strategy();
        let transition = AddressFundsTransferTransitionV0 {
            inputs: inputs.clone(),
            outputs: BTreeMap::new(),
            fee_strategy: strategy,
            user_fee_increase: TEST_FEE,
            input_witnesses: vec![],
        };
        let action =
            BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition(
                &transition, 0,
            );
        // Nonces should be identical to the input nonces
        for (addr, (original_nonce, _)) in &inputs {
            let (result_nonce, _) =
                action.inputs_with_remaining_balance.get(addr).unwrap();
            assert_eq!(*result_nonce, *original_nonce);
        }
    }

    // ---- single input ----

    #[test]
    fn test_single_input_full_deduction() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1_u32, 1000_u64));
        let strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let action = BumpAddressInputNoncesActionV0::from_inputs_with_remaining_balance(
            &inputs, &strategy, 1000, TEST_FEE,
        );
        let (_, balance) = action
            .inputs_with_remaining_balance
            .get(&PlatformAddress::P2pkh([0x11; 20]))
            .unwrap();
        assert_eq!(*balance, 0);
    }

    #[test]
    fn test_empty_inputs() {
        let inputs = BTreeMap::new();
        let strategy = vec![];
        let action = BumpAddressInputNoncesActionV0::from_inputs_with_remaining_balance(
            &inputs, &strategy, 0, 0,
        );
        assert!(action.inputs_with_remaining_balance.is_empty());
        assert_eq!(action.user_fee_increase, 0);
    }
}
