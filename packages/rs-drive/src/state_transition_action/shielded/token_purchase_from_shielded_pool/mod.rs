/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::token_purchase_from_shielded_pool::v0::TokenPurchaseFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Token purchase paid from the credit pool transition action
#[derive(Debug, Clone, From)]
pub enum TokenPurchaseFromShieldedPoolTransitionAction {
    /// v0
    V0(TokenPurchaseFromShieldedPoolTransitionActionV0),
}

impl TokenPurchaseFromShieldedPoolTransitionAction {
    /// The token whose pool the token bundle spends from or mints into
    pub fn token_id(&self) -> Identifier {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.token_id,
        }
    }

    /// Notes built 1:1 from the on-wire token bundle actions
    pub fn token_notes(&self) -> &[ShieldedActionNote] {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => &t.token_notes,
        }
    }

    /// The token pool anchor the token bundle was proven against
    pub fn token_anchor(&self) -> &[u8; 32] {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => &t.token_anchor,
        }
    }

    /// Notes built 1:1 from the on-wire credit pool fee bundle actions
    pub fn fee_notes(&self) -> &[ShieldedActionNote] {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => &t.fee_notes,
        }
    }

    /// The credit pool anchor the fee bundle was proven against
    pub fn fee_anchor(&self) -> &[u8; 32] {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => &t.fee_anchor,
        }
    }

    /// Flat fee carved from the credit pool and routed to the fee pools
    pub fn fee_amount(&self) -> Credits {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.fee_amount,
        }
    }

    /// Credit pool total balance read at transform time
    pub fn current_credit_pool_balance(&self) -> Credits {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.current_credit_pool_balance,
        }
    }

    /// The contract owner, credited with `total_agreed_price`
    pub fn contract_owner_id(&self) -> Identifier {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.contract_owner_id,
        }
    }

    /// Tokens minted into the token pool
    pub fn token_count(&self) -> TokenAmount {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.token_count,
        }
    }

    /// Credits paid to the contract owner out of the credit pool, on top of `fee_amount`
    pub fn total_agreed_price(&self) -> Credits {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.total_agreed_price,
        }
    }

    /// Whether the token may be minted for the first time by this purchase
    pub fn allow_first_mint(&self) -> bool {
        match self {
            TokenPurchaseFromShieldedPoolTransitionAction::V0(t) => t.allow_first_mint,
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
