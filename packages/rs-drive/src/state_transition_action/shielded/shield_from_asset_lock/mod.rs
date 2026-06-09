/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Shield from asset lock transition action
#[derive(Debug, Clone, From)]
pub enum ShieldFromAssetLockTransitionAction {
    /// v0
    V0(ShieldFromAssetLockTransitionActionV0),
}

impl ShieldFromAssetLockTransitionAction {
    /// Get asset lock outpoint
    pub fn asset_lock_outpoint(&self) -> &[u8; 36] {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => &transition.asset_lock_outpoint,
        }
    }
    /// Get remaining asset lock value to be consumed
    pub fn asset_lock_value_to_be_consumed(&self) -> Credits {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => {
                transition.asset_lock_value_to_be_consumed
            }
        }
    }
    /// Get signable bytes hasher
    pub fn signable_bytes_hasher(&self) -> &[u8; 32] {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => {
                &transition.signable_bytes_hasher
            }
        }
    }
    /// Get the shield amount
    pub fn shield_amount(&self) -> Credits {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => transition.shield_amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get the optional surplus-output platform address (receives the asset-lock surplus).
    pub fn surplus_output(&self) -> &Option<PlatformAddress> {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => &transition.surplus_output,
        }
    }
    /// Get the surplus credits routed to `surplus_output` (0 when `surplus_output` is `None`).
    pub fn surplus_amount(&self) -> Credits {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => transition.surplus_amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x55; 32],
            cmx: [0x66; 32],
            cv_net: [0x22; 32],
            encrypted_note: vec![0xDE, 0xAD],
        }
    }

    fn make_action() -> ShieldFromAssetLockTransitionAction {
        let v0 = ShieldFromAssetLockTransitionActionV0 {
            asset_lock_outpoint: [0xAA; 36],
            asset_lock_value_to_be_consumed: 50000,
            signable_bytes_hasher: [0xBB; 32],
            shield_amount: 45000,
            notes: vec![make_note()],
            current_total_balance: 200000,
            surplus_output: None,
            surplus_amount: 0,
        };
        ShieldFromAssetLockTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(action, ShieldFromAssetLockTransitionAction::V0(_)));
    }

    #[test]
    fn test_asset_lock_outpoint() {
        let action = make_action();
        assert_eq!(*action.asset_lock_outpoint(), [0xAA; 36]);
    }

    #[test]
    fn test_asset_lock_value_to_be_consumed() {
        let action = make_action();
        assert_eq!(action.asset_lock_value_to_be_consumed(), 50000);
    }

    #[test]
    fn test_signable_bytes_hasher() {
        let action = make_action();
        assert_eq!(*action.signable_bytes_hasher(), [0xBB; 32]);
    }

    #[test]
    fn test_shield_amount() {
        let action = make_action();
        assert_eq!(action.shield_amount(), 45000);
    }

    #[test]
    fn test_notes() {
        let action = make_action();
        let notes = action.notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].nullifier, [0x55; 32]);
        assert_eq!(notes[0].cmx, [0x66; 32]);
        assert_eq!(notes[0].encrypted_note, vec![0xDE, 0xAD]);
    }

    #[test]
    fn test_zero_values() {
        let v0 = ShieldFromAssetLockTransitionActionV0 {
            asset_lock_outpoint: [0x00; 36],
            asset_lock_value_to_be_consumed: 0,
            signable_bytes_hasher: [0x00; 32],
            shield_amount: 0,
            notes: vec![],
            current_total_balance: 0,
            surplus_output: None,
            surplus_amount: 0,
        };
        let action = ShieldFromAssetLockTransitionAction::from(v0);
        assert_eq!(action.asset_lock_value_to_be_consumed(), 0);
        assert_eq!(action.shield_amount(), 0);
        assert!(action.notes().is_empty());
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.shield_amount(), 45000);
        assert_eq!(cloned.asset_lock_value_to_be_consumed(), 50000);
        assert_eq!(cloned.notes().len(), 1);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
