mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Shielded pool to an existing identity's balance, version 0.
#[derive(Debug, Clone)]
pub struct IdentityTopUpFromShieldedPoolTransitionActionV0 {
    /// The identity whose balance is credited
    pub identity_id: Identifier,
    /// Gross amount leaving the pool; the identity receives `amount - fee_amount`
    pub amount: Credits,
    /// Notes built 1:1 from the on-wire Orchard actions
    pub notes: Vec<ShieldedActionNote>,
    /// The anchor the spend was proven against
    pub anchor: [u8; 32],
    /// Flat fee routed to the fee pools
    pub fee_amount: Credits,
    /// Pool total balance read at transform time
    pub current_total_balance: Credits,
}
