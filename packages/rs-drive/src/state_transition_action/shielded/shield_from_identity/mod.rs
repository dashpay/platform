/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shield_from_identity::v0::ShieldFromIdentityTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// Shield from identity transition action
#[derive(Debug, Clone, From)]
pub enum ShieldFromIdentityTransitionAction {
    /// v0
    V0(ShieldFromIdentityTransitionActionV0),
}

impl ShieldFromIdentityTransitionAction {
    /// The identity whose balance funds the shield
    pub fn identity_id(&self) -> Identifier {
        match self {
            ShieldFromIdentityTransitionAction::V0(transition) => transition.identity_id,
        }
    }
    /// The identity nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            ShieldFromIdentityTransitionAction::V0(transition) => transition.nonce,
        }
    }
    /// Get the shield amount
    pub fn shield_amount(&self) -> Credits {
        match self {
            ShieldFromIdentityTransitionAction::V0(transition) => transition.shield_amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldFromIdentityTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldFromIdentityTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
    /// The pool total balance read at transform time
    pub fn current_total_balance(&self) -> Credits {
        match self {
            ShieldFromIdentityTransitionAction::V0(transition) => transition.current_total_balance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            cv_net: [0x33; 32],
            encrypted_note: vec![0xAB, 0xCD, 0xEF],
        }
    }

    fn make_action() -> ShieldFromIdentityTransitionAction {
        ShieldFromIdentityTransitionAction::from(ShieldFromIdentityTransitionActionV0 {
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 3,
            shield_amount: 5000,
            notes: vec![make_note(), make_note()],
            user_fee_increase: 2,
            current_total_balance: 50000,
        })
    }

    #[test]
    fn test_accessors() {
        let action = make_action();
        assert!(matches!(action, ShieldFromIdentityTransitionAction::V0(_)));
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
        assert_eq!(action.nonce(), 3);
        assert_eq!(action.shield_amount(), 5000);
        assert_eq!(action.notes().len(), 2);
        assert_eq!(action.notes()[0].cmx, [0x22; 32]);
        assert_eq!(action.user_fee_increase(), 2);
        assert_eq!(action.current_total_balance(), 50000);
    }
}
