/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Unshield transition action
#[derive(Debug, Clone, From)]
pub enum UnshieldTransitionAction {
    /// v0
    V0(UnshieldTransitionActionV0),
}

impl UnshieldTransitionAction {
    /// Get output address
    pub fn output_address(&self) -> &PlatformAddress {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.output_address,
        }
    }
    /// Get amount
    pub fn amount(&self) -> Credits {
        match self {
            UnshieldTransitionAction::V0(transition) => transition.amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Fee amount (value_balance - amount), paid to proposers
    pub fn fee_amount(&self) -> Credits {
        match self {
            UnshieldTransitionAction::V0(transition) => transition.fee_amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x33; 32],
            cmx: [0x44; 32],
            encrypted_note: vec![0x10, 0x20],
        }
    }

    fn make_action() -> UnshieldTransitionAction {
        let v0 = UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 7500,
            notes: vec![make_note()],
            anchor: [0x55; 32],
            fee_amount: 250,
            current_total_balance: 100000,
        };
        UnshieldTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(action, UnshieldTransitionAction::V0(_)));
    }

    #[test]
    fn test_output_address() {
        let action = make_action();
        assert_eq!(*action.output_address(), PlatformAddress::P2pkh([0xBB; 20]));
    }

    #[test]
    fn test_amount() {
        let action = make_action();
        assert_eq!(action.amount(), 7500);
    }

    #[test]
    fn test_notes() {
        let action = make_action();
        let notes = action.notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].nullifier, [0x33; 32]);
        assert_eq!(notes[0].cmx, [0x44; 32]);
    }

    #[test]
    fn test_anchor() {
        let action = make_action();
        assert_eq!(*action.anchor(), [0x55; 32]);
    }

    #[test]
    fn test_fee_amount() {
        let action = make_action();
        assert_eq!(action.fee_amount(), 250);
    }

    #[test]
    fn test_zero_fee_amount() {
        let v0 = UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2sh([0x00; 20]),
            amount: 0,
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 0,
            current_total_balance: 0,
        };
        let action = UnshieldTransitionAction::from(v0);
        assert_eq!(action.amount(), 0);
        assert_eq!(action.fee_amount(), 0);
        assert!(action.notes().is_empty());
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.amount(), 7500);
        assert_eq!(cloned.fee_amount(), 250);
        assert_eq!(*cloned.anchor(), [0x55; 32]);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
