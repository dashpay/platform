mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::identity::Identity;

/// IdentityCreateFromShieldedPool transition action v0
#[derive(Debug, Clone)]
pub struct IdentityCreateFromShieldedPoolTransitionActionV0 {
    /// The fully-built new identity (id derived from the spend nullifiers, keys from the
    /// transition's `public_keys`). Its balance is the FULL `denomination`; the fee is moved
    /// from this balance into the fee pools at execution, so the identity ends with
    /// `denomination - fee_amount`.
    pub identity: Identity,
    /// Notes from the orchard bundle actions (nullifiers to insert + change notes to append).
    pub notes: Vec<ShieldedActionNote>,
    /// The anchor used for verification.
    pub anchor: [u8; 32],
    /// The fixed exit denomination (in credits) leaving the shielded pool. Equals the new
    /// identity's initial balance and the amount the shielded pool is decremented by (a move
    /// between two balance trees — no change to the system-credit supply).
    pub denomination: Credits,
    /// Total fee (metered GroveDB write cost + flat shielded verification/compute fee) moved from
    /// the new identity's balance into the fee pools at execution. MUST be `< denomination`.
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool (decremented by `denomination`).
    pub current_total_balance: Credits,
}
