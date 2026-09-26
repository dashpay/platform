mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Token shielded transfer paid from the credit pool, version 0: the token pool side and the credit pool fee side of the spend.
#[derive(Debug, Clone)]
pub struct TokenShieldedTransferWithShieldedFeeTransitionActionV0 {
    /// The token whose pool the token bundle spends from or mints into
    pub token_id: Identifier,
    /// Notes built 1:1 from the on-wire token bundle actions
    pub token_notes: Vec<ShieldedActionNote>,
    /// The token pool anchor the token bundle was proven against
    pub token_anchor: [u8; 32],
    /// Notes built 1:1 from the on-wire credit pool fee bundle actions
    pub fee_notes: Vec<ShieldedActionNote>,
    /// The credit pool anchor the fee bundle was proven against
    pub fee_anchor: [u8; 32],
    /// Flat fee carved from the credit pool and routed to the fee pools
    pub fee_amount: Credits,
    /// Credit pool total balance read at transform time
    pub current_credit_pool_balance: Credits,
}
