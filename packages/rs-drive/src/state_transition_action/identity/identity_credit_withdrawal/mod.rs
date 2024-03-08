/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use derive_more::From;
use dpp::document::Document;

use dpp::platform_value::Identifier;
use dpp::prelude::{UserFeeIncrease, IdentityNonce};

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
            IdentityCreditWithdrawalTransitionAction::V0(transition) => transition.fee_multiplier,
        }
    }
}
