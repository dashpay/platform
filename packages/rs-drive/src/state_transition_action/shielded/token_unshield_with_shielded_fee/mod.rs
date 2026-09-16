/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::token_unshield_with_shielded_fee::v0::TokenUnshieldWithShieldedFeeTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Identity-less unshield transition action
#[derive(Debug, Clone, From)]
pub enum TokenUnshieldWithShieldedFeeTransitionAction {
    /// v0
    V0(TokenUnshieldWithShieldedFeeTransitionActionV0),
}

impl TokenUnshieldWithShieldedFeeTransitionAction {
    /// The token whose pool the token bundle spends from or mints into
    pub fn token_id(&self) -> Identifier {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => t.token_id,
        }
    }

    /// Notes built 1:1 from the on-wire token bundle actions
    pub fn token_notes(&self) -> &[ShieldedActionNote] {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => &t.token_notes,
        }
    }

    /// The token pool anchor the token bundle was proven against
    pub fn token_anchor(&self) -> &[u8; 32] {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => &t.token_anchor,
        }
    }

    /// Notes built 1:1 from the on-wire credit pool fee bundle actions
    pub fn fee_notes(&self) -> &[ShieldedActionNote] {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => &t.fee_notes,
        }
    }

    /// The credit pool anchor the fee bundle was proven against
    pub fn fee_anchor(&self) -> &[u8; 32] {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => &t.fee_anchor,
        }
    }

    /// Flat fee carved from the credit pool and routed to the fee pools
    pub fn fee_amount(&self) -> Credits {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => t.fee_amount,
        }
    }

    /// Credit pool total balance read at transform time
    pub fn current_credit_pool_balance(&self) -> Credits {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => t.current_credit_pool_balance,
        }
    }

    /// The identity whose token balance receives `amount`
    pub fn recipient_id(&self) -> Identifier {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => t.recipient_id,
        }
    }

    /// Tokens leaving the token pool into the recipient's balance
    pub fn amount(&self) -> TokenAmount {
        match self {
            TokenUnshieldWithShieldedFeeTransitionAction::V0(t) => t.amount,
        }
    }

    /// The nullifiers the token bundle spends
    pub fn token_nullifiers(&self) -> Vec<[u8; 32]> {
        self.token_notes()
            .iter()
            .map(|note| note.nullifier)
            .collect()
    }
}
