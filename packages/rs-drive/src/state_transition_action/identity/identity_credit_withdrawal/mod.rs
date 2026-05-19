/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use derive_more::From;
use dpp::document::Document;

use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityCreditWithdrawalTransitionAction {
    /// v0
    V0(IdentityCreditWithdrawalTransitionActionV0),
}

impl IdentityCreditWithdrawalTransitionAction {
    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Amount
    pub fn amount(&self) -> u64 {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => transition.amount,
        }
    }

    /// Recipient Id
    pub fn prepared_withdrawal_document(&self) -> &Document {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => {
                &transition.prepared_withdrawal_document
            }
        }
    }

    /// Recipient Id
    pub fn prepared_withdrawal_document_owned(self) -> Document {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => {
                transition.prepared_withdrawal_document
            }
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => {
                transition.user_fee_increase
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
    use dpp::document::DocumentV0;

    fn make_document() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from([0xDD; 32]),
            owner_id: Identifier::from([0xAA; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    fn make_v0() -> IdentityCreditWithdrawalTransitionActionV0 {
        IdentityCreditWithdrawalTransitionActionV0 {
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 10,
            prepared_withdrawal_document: make_document(),
            amount: 7500,
            user_fee_increase: 6,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityCreditWithdrawalTransitionAction = v0.into();
        assert!(matches!(
            action,
            IdentityCreditWithdrawalTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_nonce() {
        let action = IdentityCreditWithdrawalTransitionAction::V0(make_v0());
        assert_eq!(action.nonce(), 10);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityCreditWithdrawalTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_amount() {
        let action = IdentityCreditWithdrawalTransitionAction::V0(make_v0());
        assert_eq!(action.amount(), 7500);
    }

    #[test]
    fn test_prepared_withdrawal_document() {
        let action = IdentityCreditWithdrawalTransitionAction::V0(make_v0());
        let doc = action.prepared_withdrawal_document();
        match doc {
            Document::V0(v0) => {
                assert_eq!(v0.id, Identifier::from([0xDD; 32]));
                assert_eq!(v0.owner_id, Identifier::from([0xAA; 32]));
            }
        }
    }

    #[test]
    fn test_prepared_withdrawal_document_owned() {
        let action = IdentityCreditWithdrawalTransitionAction::V0(make_v0());
        let doc = action.prepared_withdrawal_document_owned();
        match doc {
            Document::V0(v0) => {
                assert_eq!(v0.id, Identifier::from([0xDD; 32]));
            }
        }
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityCreditWithdrawalTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 6);
    }
}
