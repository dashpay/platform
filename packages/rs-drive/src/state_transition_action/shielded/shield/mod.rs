/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shield::v0::ShieldTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// Shield transition action
#[derive(Debug, Clone, From)]
pub enum ShieldTransitionAction {
    /// v0
    V0(ShieldTransitionActionV0),
}

impl ShieldTransitionAction {
    /// Get inputs with remaining balance
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            ShieldTransitionAction::V0(transition) => &transition.inputs_with_remaining_balance,
        }
    }
    /// Get the shield amount
    pub fn shield_amount(&self) -> Credits {
        match self {
            ShieldTransitionAction::V0(transition) => transition.shield_amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            ShieldTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![0xAB, 0xCD, 0xEF],
        }
    }

    fn make_action() -> ShieldTransitionAction {
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(addr, (3_u32, 10000_u64));

        let v0 = ShieldTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            shield_amount: 5000,
            notes: vec![make_note(), make_note()],
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 2,
            current_total_balance: 50000,
        };
        ShieldTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(action, ShieldTransitionAction::V0(_)));
    }

    #[test]
    fn test_inputs_with_remaining_balance() {
        let action = make_action();
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 1);
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        let (nonce, balance) = inputs.get(&addr).unwrap();
        assert_eq!(*nonce, 3);
        assert_eq!(*balance, 10000);
    }

    #[test]
    fn test_shield_amount() {
        let action = make_action();
        assert_eq!(action.shield_amount(), 5000);
    }

    #[test]
    fn test_notes() {
        let action = make_action();
        let notes = action.notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].nullifier, [0x11; 32]);
        assert_eq!(notes[0].cmx, [0x22; 32]);
        assert_eq!(notes[0].encrypted_note, vec![0xAB, 0xCD, 0xEF]);
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 2);
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
    fn test_empty_notes() {
        let v0 = ShieldTransitionActionV0 {
            inputs_with_remaining_balance: BTreeMap::new(),
            shield_amount: 0,
            notes: vec![],
            fee_strategy: vec![],
            user_fee_increase: 0,
            current_total_balance: 0,
        };
        let action = ShieldTransitionAction::from(v0);
        assert!(action.notes().is_empty());
        assert_eq!(action.shield_amount(), 0);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.shield_amount(), 5000);
        assert_eq!(cloned.notes().len(), 2);
        assert_eq!(cloned.user_fee_increase(), 2);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
