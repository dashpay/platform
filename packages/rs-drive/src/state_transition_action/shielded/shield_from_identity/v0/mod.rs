mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// Identity balance to shielded pool, version 0.
#[derive(Debug, Clone)]
pub struct ShieldFromIdentityTransitionActionV0 {
    /// The identity whose balance funds the shield
    pub identity_id: Identifier,
    /// The identity nonce to store
    pub nonce: IdentityNonce,
    /// Credits removed from the identity and added to the pool
    pub shield_amount: Credits,
    /// Notes built 1:1 from the on-wire Orchard actions
    pub notes: Vec<ShieldedActionNote>,
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Pool total balance read at transform time
    pub current_total_balance: Credits,
}
