/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityCreditTransferTransitionAction {
    /// v0
    V0(IdentityCreditTransferTransitionActionV0),
}

impl IdentityCreditTransferTransitionAction {
    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// Transfer amount
    pub fn transfer_amount(&self) -> Credits {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.transfer_amount,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Recipient Id
    pub fn recipient_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.recipient_id,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;

    fn make_v0() -> IdentityCreditTransferTransitionActionV0 {
        IdentityCreditTransferTransitionActionV0 {
            transfer_amount: 5000,
            recipient_id: Identifier::from([0xBB; 32]),
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 42,
            user_fee_increase: 3,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityCreditTransferTransitionAction = v0.into();
        assert!(matches!(
            action,
            IdentityCreditTransferTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_nonce() {
        let action = IdentityCreditTransferTransitionAction::V0(make_v0());
        assert_eq!(action.nonce(), 42);
    }

    #[test]
    fn test_transfer_amount() {
        let action = IdentityCreditTransferTransitionAction::V0(make_v0());
        assert_eq!(action.transfer_amount(), 5000);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityCreditTransferTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_recipient_id() {
        let action = IdentityCreditTransferTransitionAction::V0(make_v0());
        assert_eq!(action.recipient_id(), Identifier::from([0xBB; 32]));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityCreditTransferTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 3);
    }
}
