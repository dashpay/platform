/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::fee::Credits;

/// Shielded transfer transition action
#[derive(Debug, Clone, From)]
pub enum ShieldedTransferTransitionAction {
    /// v0
    V0(ShieldedTransferTransitionActionV0),
}

impl ShieldedTransferTransitionAction {
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Get fee amount
    pub fn fee_amount(&self) -> Credits {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => transition.fee_amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0xAA; 32],
            cmx: [0xBB; 32],
            cv_net: [0x22; 32],
            encrypted_note: vec![0x01, 0x02, 0x03],
        }
    }

    fn make_action() -> ShieldedTransferTransitionAction {
        let v0 = ShieldedTransferTransitionActionV0 {
            notes: vec![make_note(), make_note(), make_note()],
            anchor: [0xCC; 32],
            fee_amount: 100,
            current_total_balance: 999999,
        };
        ShieldedTransferTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(action, ShieldedTransferTransitionAction::V0(_)));
    }

    #[test]
    fn test_notes() {
        let action = make_action();
        let notes = action.notes();
        assert_eq!(notes.len(), 3);
        for note in notes {
            assert_eq!(note.nullifier, [0xAA; 32]);
            assert_eq!(note.cmx, [0xBB; 32]);
            assert_eq!(note.encrypted_note, vec![0x01, 0x02, 0x03]);
        }
    }

    #[test]
    fn test_anchor() {
        let action = make_action();
        assert_eq!(*action.anchor(), [0xCC; 32]);
    }

    #[test]
    fn test_fee_amount() {
        let action = make_action();
        assert_eq!(action.fee_amount(), 100);
    }

    #[test]
    fn test_empty_transfer() {
        let v0 = ShieldedTransferTransitionActionV0 {
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 0,
            current_total_balance: 0,
        };
        let action = ShieldedTransferTransitionAction::from(v0);
        assert!(action.notes().is_empty());
        assert_eq!(*action.anchor(), [0x00; 32]);
        assert_eq!(action.fee_amount(), 0);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.notes().len(), 3);
        assert_eq!(*cloned.anchor(), [0xCC; 32]);
        assert_eq!(cloned.fee_amount(), 100);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
