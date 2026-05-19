/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum AddressFundsTransferTransitionAction {
    /// v0
    V0(AddressFundsTransferTransitionActionV0),
}

impl AddressFundsTransferTransitionAction {
    /// Get inputs
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }
    /// Get outputs
    pub fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => &transition.outputs,
        }
    }
    /// Returns owned copies of inputs and outputs.
    pub fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    ) {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => {
                (transition.inputs_with_remaining_balance, transition.outputs)
            }
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;

    fn make_action() -> AddressFundsTransferTransitionAction {
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_b = PlatformAddress::P2pkh([0xBB; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_a, (1_u32, 5000_u64));

        let mut outputs = BTreeMap::new();
        outputs.insert(addr_b, 3000_u64);

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let v0 = AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            outputs,
            fee_strategy,
            user_fee_increase: 10,
        };
        AddressFundsTransferTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(
            action,
            AddressFundsTransferTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_inputs_with_remaining_balance() {
        let action = make_action();
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 1);
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);
        let (nonce, balance) = inputs.get(&addr_a).unwrap();
        assert_eq!(*nonce, 1);
        assert_eq!(*balance, 5000);
    }

    #[test]
    fn test_outputs() {
        let action = make_action();
        let outputs = action.outputs();
        assert_eq!(outputs.len(), 1);
        let addr_b = PlatformAddress::P2pkh([0xBB; 20]);
        assert_eq!(*outputs.get(&addr_b).unwrap(), 3000_u64);
    }

    #[test]
    fn test_inputs_and_outputs_owned() {
        let action = make_action();
        let (inputs, outputs) = action.inputs_with_remaining_balance_and_outputs_owned();
        assert_eq!(inputs.len(), 1);
        assert_eq!(outputs.len(), 1);
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_b = PlatformAddress::P2pkh([0xBB; 20]);
        assert!(inputs.contains_key(&addr_a));
        assert!(outputs.contains_key(&addr_b));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 10);
    }

    #[test]
    fn test_fee_strategy() {
        let action = make_action();
        let strategy = action.fee_strategy();
        assert_eq!(strategy.len(), 1);
        assert!(matches!(
            strategy[0],
            AddressFundsFeeStrategyStep::DeductFromInput(0)
        ));
    }

    #[test]
    fn test_default_v0() {
        let v0 = AddressFundsTransferTransitionActionV0::default();
        let action = AddressFundsTransferTransitionAction::from(v0);
        assert!(action.inputs_with_remaining_balance().is_empty());
        assert!(action.outputs().is_empty());
        assert!(action.fee_strategy().is_empty());
        assert_eq!(action.user_fee_increase(), 0);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.user_fee_increase(), 10);
        assert_eq!(cloned.inputs_with_remaining_balance().len(), 1);
        assert_eq!(cloned.outputs().len(), 1);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
